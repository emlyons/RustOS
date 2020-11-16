use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use alloc::vec;
use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;

use crate::allocator;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;

use aarch64::vmsa::*;
use shim::const_assert_size;

const TABLE_SIZE: usize = PAGE_SIZE / size_of::<u64>();

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; TABLE_SIZE],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
	L2PageTable {entries: [RawL2Entry::new(0); TABLE_SIZE]}
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
	PhysicalAddr::from(self as *const _ as usize)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
	L3Entry(RawL3Entry::new(0))
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
	self.0.get_value(RawL2Entry::VALID) != 0
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
	match self.is_valid() {
            true => Some(PhysicalAddr::from(self.0.get_value(RawL2Entry::ADDR) << PAGE_ALIGN)),
	    false => None,
	}
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; TABLE_SIZE],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
	L3PageTable {entries: [L3Entry::new(); TABLE_SIZE]}
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const _ as usize)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [Box<L3PageTable>; 3],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {

	let mut table = Box::new(PageTable {
            l2: L2PageTable::new(),
            l3: [Box::new(L3PageTable::new()), Box::new(L3PageTable::new()), Box::new(L3PageTable::new())],
        });

	for (index, l3) in table.l3.iter().enumerate() {
	    let entry = &mut table.l2.entries[index];
	    entry.set_value(l3.as_ptr().as_u64() >> PAGE_ALIGN, RawL2Entry::ADDR);	    
	    entry.set_value(1, RawL2Entry::AF);
	    entry.set_value(EntrySh::ISh, RawL2Entry::SH);
	    entry.set_value(perm, RawL2Entry::AP);
	    entry.set_value(1, RawL2Entry::NS);
	    entry.set_value(EntryAttr::Mem, RawL2Entry::ATTR);
	    entry.set_value(EntryType::Table, RawL2Entry::TYPE);
	    entry.set_value(EntryValid::Valid, RawL2Entry::VALID);
	}
        table
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// L2index should be smaller than the number of L3PageTable.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
	assert_eq!(va.as_u64() as usize % PAGE_SIZE, 0);
	let num_l3 = 2;
	let il3 = (va.as_u64() >> 16) & 0x1FFF;
	let il2 = ((va.as_u64() >> 16) >> 13) & 0x01;

	assert!(il2 < num_l3);

	(il2 as usize, il3 as usize)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2, l3) = PageTable::locate(va);
        self.l3[l2].entries[l3].is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        let (l2, l3) = PageTable::locate(va);
        self.l3[l2].entries[l3].0 = entry;
        self
    }

    pub fn get_entry(&self, va: VirtualAddr) -> &L3Entry {
        let (l2, l3) = PageTable::locate(va);
        &self.l3[l2].entries[l3]
    }

    pub fn get_entry_mut(&mut self, va: VirtualAddr) -> &mut L3Entry {
        let (l2, l3) = PageTable::locate(va);
        &mut self.l3[l2].entries[l3]
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }
}

impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;    
    type IntoIter = Chain<Iter<'a, L3Entry>, Iter<'a, L3Entry>>;
    
    fn into_iter(self) -> Self::IntoIter {
	self.l3[0].entries.iter().chain(self.l3[1].entries.iter())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
	
	let mut kpt: Box<PageTable> = PageTable::new(EntryPerm::KERN_RW);
	let (mut mem_start, mut mem_end) = allocator::memory_map().unwrap();
	let mem_start = 0;
	let mem_end = mem_end >> PAGE_ALIGN;
	let io_start = IO_BASE >> PAGE_ALIGN;
	let io_end = IO_BASE_END >> PAGE_ALIGN;
	
	assert!(mem_end <= IO_BASE);
	assert!(kpt.l3.len() * TABLE_SIZE >= io_end);
	
	// kernel memory is mapped 1:1
	for i in mem_start..mem_end {
	    let index_l3 = i / TABLE_SIZE;
	    let index_entry = i % TABLE_SIZE;
	    
	    let entry: &mut RawL3Entry = &mut kpt.l3[index_l3].entries[index_entry].0;
	    
	    entry.set_value(i as u64, RawL2Entry::ADDR);
	    entry.set_value(1, RawL2Entry::AF);
	    entry.set_value(EntrySh::ISh, RawL2Entry::SH);
	    entry.set_value(EntryPerm::KERN_RW, RawL2Entry::AP);
	    entry.set_value(1, RawL2Entry::NS);
	    entry.set_value(EntryAttr::Mem, RawL2Entry::ATTR);
	    entry.set_value(PageType::Page, RawL2Entry::TYPE);
	    entry.set_value(EntryValid::Valid, RawL2Entry::VALID);
	}

	// kernel i/o is mapped 1:1
	for i in io_start..io_end {
	    let index_l3 = i / TABLE_SIZE;
	    let index_entry = i % TABLE_SIZE;
	    let entry: &mut RawL3Entry = &mut kpt.l3[index_l3].entries[index_entry].0;
	    
	    entry.set_value(i as u64, RawL2Entry::ADDR);
	    entry.set_value(1, RawL2Entry::AF);
	    entry.set_value(EntrySh::OSh, RawL2Entry::SH);
	    entry.set_value(EntryPerm::KERN_RW, RawL2Entry::AP);
	    entry.set_value(1, RawL2Entry::NS);
	    entry.set_value(EntryAttr::Dev, RawL2Entry::ATTR);
	    entry.set_value(PageType::Page, RawL2Entry::TYPE);
	    entry.set_value(EntryValid::Valid, RawL2Entry::VALID);
	}
	KernPageTable(kpt)
    }

    pub fn get_baddr(&self) -> PhysicalAddr {
        self.0.get_baddr()
    }

}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
	UserPageTable(PageTable::new(EntryPerm::USER_RW))
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
	assert!(va.as_usize() >= USER_IMG_BASE);

	// retrieve entry
	if self.0.is_valid(va) {
	    panic!("attempt to reallocate virtual address");
	}
	let phys_page: *mut u8 = unsafe{
	    ALLOCATOR.alloc(Page::layout())
	};

	let phys_addr = (phys_page as u64) >> PAGE_ALIGN;	    
	let mut entry: RawL3Entry = RawL3Entry::new(0);
	entry.set_value(phys_addr, RawL3Entry::ADDR);
	entry.set_value(1, RawL2Entry::AF);
	entry.set_value(EntrySh::ISh, RawL3Entry::SH);
	entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);
	entry.set_value(1, RawL2Entry::NS);
	entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
	entry.set_value(PageType::Page, RawL3Entry::TYPE);
	entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
	self.0.set_entry(va, entry);

	unsafe{
	    core::slice::from_raw_parts_mut(phys_page, PAGE_SIZE)
	}
    }

    pub fn get_page(&mut self, va: VirtualAddr) -> PhysicalAddr {
	let (l2, l3) = PageTable::locate(va);
        let entry: L3Entry = self.l3[l2].entries[l3];
	let addr = entry.get_page_addr().unwrap();
	return addr;
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FIXME: Implement `Drop` for `UserPageTable`.
impl Drop for UserPageTable {
    fn drop(&mut self) {
	for entry in self.0.into_iter() {
	    if let Some(mut phys_addr) = entry.get_page_addr() {
		unsafe{
		    ALLOCATOR.dealloc(phys_addr.as_mut_ptr(), Page::layout());
		};
	    }
	}
    }
}

// FIXME: Implement `fmt::Debug` as you need.
impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("User Page Table")
            .field("base address", &self.get_baddr())
            .finish()
    }
}

impl fmt::Debug for KernPageTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Kernel Page Table")
            .field("base address", &self.get_baddr())
            .finish()
    }
}
