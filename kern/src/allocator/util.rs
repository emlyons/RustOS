use core::cmp::Ord;
use core::ops::Add;

/// Fast algorithm for checking if log2(num) is integer aligned (a power of 2)
/// runs in O(1)
fn is_pow_two(num: usize) -> Result<(),()> {
    match (num & (num-1)) == 0 {
	true => Ok(()),
	false => Err(()),
    }
}

/// Checks wether range of num can support the given offset without rapping
/// in debug arithmetic wrapping will panic
/// in release wrapping will occur as two's complement
fn is_overflow(num: usize, offset: usize) -> Result<(),()> {
    let sum = num + offset;
    match sum < num {
	true => Err(()),
	false => Ok(()),
    }
}

/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    is_pow_two(align).unwrap();
    
    let remainder: usize = addr % align;
    addr - remainder
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    is_pow_two(align).unwrap();

    let remainder: usize = addr % align;
    let offset = (align - remainder) % align;

    is_overflow(addr, offset).unwrap();
    
    addr + offset
}
