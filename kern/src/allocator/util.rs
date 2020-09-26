/// Checks if NUM is integer an integer power of two
pub fn is_power_of_two(num: usize) -> bool {
    num.next_power_of_two() == num
}

/// Checks whether range of NUM can support the given offset without wrapping
fn is_overflow(num: usize, offset: usize) -> bool {
    let (sum, overflow) = num.overflowing_add(offset);
    overflow
}

/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    assert!(is_power_of_two(align));
    
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
    assert!(is_power_of_two(align));

    let remainder: usize = addr % align;
    let offset: usize = (align - remainder) % align;

    assert!(!is_overflow(addr, offset));
    
    let align_addr: usize = addr + offset;

    return align_addr;
}
