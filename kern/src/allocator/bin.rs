use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::cmp;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

use crate::console::kprintln;

const ALLOC_BOUND: usize = 64;

/// returns index such that Allocator.align[index] is the lowest index for which the alignment requirement is satisfied
/// align is a byte value, the hash returns an index into the align member of an Allocator struct
/// align is assumed to be a power of two
fn align_hash (align: usize) -> usize {
    let hash = align.next_power_of_two().trailing_zeros() as usize;
    hash
}

/// returns index such that Allocator.align[X][index] is tightest bounded block on the size requirement
/// size is a byte value, the hash returns an index into an element of an align member of an Allocator struct
fn size_hash (size: usize) -> usize {
    let hash = cmp::max(size.next_power_of_two().trailing_zeros() as usize, 3) - 3;
    hash
}

/// returns the largest power of two for which addr is aligned
///
/// # Panics
///
/// Panics if `addr` is not aligned to a power of 2.
pub fn strongest_align (addr: usize) -> usize {
    addr.trailing_zeros() as usize
}

fn bump(current: usize, end: usize, align: usize, size: usize) -> Option<(usize, usize)> {
    let aligned_addr = align_up(current, align);
    let (next, overflow) = aligned_addr.overflowing_add(size.next_power_of_two());
    let size = next - current;
    
    // not enough space
    if (next > end) || overflow {
	None
    }
    else {
	Some((aligned_addr, size))
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
    start: usize,
    end: usize,
    free_block: [[LinkedList; ALLOC_BOUND]; ALLOC_BOUND],
    free_hole: LinkedList,
    frag_count: usize,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
	Allocator {
	    current: start,
	    start: start,
	    end: end,
	    free_block: [[LinkedList::new(); ALLOC_BOUND]; ALLOC_BOUND],
	    free_hole: LinkedList::new(),
	    frag_count: 0,
	}
    }

    /// returns a block of the minimum bounding size that meets alignment requirement
    /// searches for a free block in the allocation struct
    /// if no block exists creates new block from free memory
    fn get_block(&mut self, layout: Layout) -> Option<*mut u8> {
	let mut align_index = align_hash(layout.align());
	let bin_index = size_hash(layout.size());
	let bin_size = layout.size().next_power_of_two();
	kprintln!("Alloc: align: {}  bin: {}", align_index, bin_index);
	
	// search for existing block
	while align_index < ALLOC_BOUND {
	    if self.free_block[align_index][bin_index].is_empty() {
		align_index += 1;
	    } else {
		kprintln!("block existed");
		return Some(self.free_block[align_index][bin_index].pop().unwrap() as *mut u8);
	    }
	}

	// search for free hole
	if let Some(addr) = self.get_from_free_hole(layout.align(), bin_size) {
	    kprintln!("free hole used");
	    return Some(addr);
	}
	
	// no existing block
	kprintln!("block made");
	self.make_block(layout)
    }

    /// allocates blocks of layout.SIZE and inserts into Allocator stryct until one that meets alignment requirement is made
    /// The aligned block is not inserted but a pointer to the block is returned
    fn make_block(&mut self, layout: Layout) -> Option<*mut u8> {
	let size: usize = layout.size();
	let align: usize = layout.align();
	
	// mearest aligned address
	if let Some((block_addr, block_size)) = bump(self.current, self.end, align, size) {
	
	    // save unallocated memory to free hole list
	    self.frag_count += self.save_free_hole(self.current, block_addr - self.current);

	    // return aligned block
	    self.current = block_addr + block_size;
	    Some(block_addr as *mut u8)
	}
	else {

	    unreachable!("\n\n\n stack was exhausted, FRAG_COUNT = {} \n HEAP_SIZE = {}  \n\n\n", self.frag_count, self.end - self.start);
	    return None;
	}
    }
    
    fn insert_block(&mut self, ptr: *mut u8, layout: Layout) {
	let align_index = strongest_align(ptr as usize);
	let bin_index = size_hash(layout.size());
	assert!(align_index >= align_hash(layout.align()));
	assert!(!ptr.is_null());
	kprintln!("Dealloc: align: {}  bin: {}", align_index, bin_index);
	unsafe {self.free_block[align_index][bin_index].push(ptr as *mut usize);}
    }

    fn save_free_hole(&mut self, addr: usize, mut size: usize) -> usize {
	if size >= 16 {
	    unsafe {
		*((addr + 8) as *mut usize) = size;
		self.free_hole.push(addr as *mut usize);
		size = 0;
	    }
	}
	else if size >= 8 {
	    let align = usize::pow(2, strongest_align(addr) as u32);
	    let hole_layout = Layout::from_size_align(size, align).unwrap();
	    self.insert_block(addr as *mut u8, hole_layout);
	    size - 8;
	}
	return size;
    }

    fn get_from_free_hole(&mut self, align: usize, size: usize) -> Option<*mut u8> {

	let mut block: Option<*mut u8> = None;
	let mut used_free = LinkedList::new();

	while !self.free_hole.is_empty() {
	    let free_hole = self.free_hole.pop().unwrap();
	    let free_size = unsafe {*((free_hole as usize + 8) as *mut usize)};

	    if let Some((align_addr, align_size)) = bump(free_hole as usize, free_hole as usize + free_size, align, size) {
		let pre_start = free_hole as usize;
		let pre_size = align_addr - pre_start;
		self.frag_count += self.save_free_hole(pre_start, pre_size);
		let post_start = align_addr + align_size;
		let post_size = (free_hole as usize + free_size) - post_start;
		self.frag_count += self.save_free_hole(post_start, post_size);
		block = Some(align_addr as *mut u8);
		break;
	    }    
	    unsafe{used_free.push(free_hole)};
	}

	// replace removed free holes
	while !used_free.is_empty() {
	    let free_hole = used_free.pop().unwrap();	    
	    unsafe {self.free_hole.push(free_hole)};
	}
	return block;
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
	match self.get_block(layout) {
	    Some(addr) => addr,
	    _ => ptr::null_mut(),
	}
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
