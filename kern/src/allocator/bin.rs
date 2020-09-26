use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::cmp;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

use crate::console::kprintln;

const ALLOC_BOUND: usize = 64 - 3;

/// returns index such that Allocator.align[X][index] is tightest bounded block on the size requirement
/// size is a byte value, the hash returns an index into an element of an align member of an Allocator struct
fn get_bin (size: usize) -> usize {
    let bin_size = get_bin_size(size);
    cmp::max(bin_size.trailing_zeros() as usize, 3) - 3
}

/// returns the adjusted bin size, bins are power of two sized to reduce fragmentation
fn get_bin_size (size: usize) -> usize {
    size.next_power_of_two()
}

fn is_align (addr: usize, align: usize) -> bool {
    (addr % align) == 0
}

fn bump(current: usize, end: usize, size: usize, align: usize) -> Option<usize> {
    let aligned_addr = align_up(current, align);
    let (next, overflow) = aligned_addr.overflowing_add(size);
    
    // not enough space
    if (next > end) || overflow {
	None
    }
    else {
	Some(aligned_addr)
    }	    
}

/// A simple allocator that allocates based on size classes.
/// align[N]             : N -> aligned to 2^N bytes
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^32 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   

pub struct Allocator {
    // FIXME: Add the necessary fields.
    current: usize,
    end: usize,
    free_block: [LinkedList; ALLOC_BOUND],
    unused: LinkedList,
    frag_count: usize,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
	Allocator {
	    current: start,
	    end: end,
	    free_block: [LinkedList::new(); ALLOC_BOUND],
	    unused: LinkedList::new(),
	    frag_count: 0,
	}
    }

    /// examines the list of externally fragmented memory for a block meeting allocation requirements
    /// if a black is found it is removed from the list
    /// the truncated region is partitioned into the new block and the remaning fragments the latter of which is reinserted to the fragmented memory list
    fn find_block_external_frag (&mut self, size: usize, align: usize) -> Option<*mut u8> {

	let mut block: Option<*mut u8> = None;
	let mut inspect_list = LinkedList::new();

	while !self.unused.is_empty() {
	    let frag = self.unused.pop().unwrap();
	    let frag_size = unsafe {*((frag as usize + 8) as *mut usize)};

	    if let Some(align_addr) = bump(frag as usize, frag as usize + frag_size, size, align) {
		// save preceding fragment
		let pre_start = frag as usize;
		let pre_size = align_addr - pre_start;
		self.save_external_frag(pre_start, pre_size);

		// save proceding fragment
		let post_start = align_addr + size;
		let post_size = (frag as usize + frag_size) - post_start;
		self.save_external_frag(post_start, post_size);

		block = Some(align_addr as *mut u8);
		break;
	    }
	    unsafe{inspect_list.push(frag)};
	}

	// replace removed free holes
	while !inspect_list.is_empty() {
	    let frag = inspect_list.pop().unwrap();	    
	    unsafe {self.unused.push(frag)};
	}
	return block;
    }

    /// saves reference to region lost due to alignment constraints on allocation of new blocks
    /// these unused regions are check in the future as a last effort before allocating new memory
    fn save_external_frag (&mut self, start: usize, size: usize) {
	if size >= 16 {
	    unsafe {
		*((start + 8) as *mut usize) = size;
		self.unused.push(start as *mut usize);
	    }
	}
	else if size >= 8 {
	    let bin_index = get_bin(8);
	    unsafe {
		self.free_block[bin_index].push(start as *mut usize);
	    }
	    self.frag_count += size - 8;
	}
	else {
	    self.frag_count += size;
	}
	self.frag_count += size;
    }

    /// adds a block to free block structure
    /// this is called on deallocation and assumes the block is no longer in use by the caller
    fn insert_block(&mut self, ptr: *mut u8, layout: Layout) {
	let size = cmp::max(layout.size(), layout.align());
	let bin_index = get_bin(size);

	unsafe {
	    self.free_block[bin_index].push(ptr as *mut usize);
	}
    }
}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
	let size = cmp::max(layout.size(), layout.align());
	let bin_index = get_bin(size);

	// search for reusable block
	for block in self.free_block[bin_index].iter_mut() {
	    if is_align(block.value() as usize, layout.align()) {
		kprintln!("existing block");
		return block.pop() as *mut u8;
	    }
	}

	// search for block in externally fragmented memory
	if let Some(addr) = self.find_block_external_frag(size, layout.align()) {
	    return addr as * mut u8;
	}

	// if no block bump allocate more memory
	if let Some(addr) = bump(self.current, self.end, size, layout.align()) {
	    self.save_external_frag(self.current, addr - self.current);
	    self.current = addr + size;
	    return addr as *mut u8;
	}

	// exhausted
	ptr::null_mut()
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
	self.insert_block(ptr, layout);
    }
}
