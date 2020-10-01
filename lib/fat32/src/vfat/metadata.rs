use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    // FIXME: Fill me in.
}

fn truncate_bits(val: u16, least_sigbit: u16, num_bits: u16) -> u16 {
    assert!(num_bits > 0);
    assert!(least_sigbit + num_bits <= 16);
    let mask: u16 = 0xFFFF >> 16 - num_bits;
    let shift_down: u16 = least_sigbit;
    let masked_val = (val >> least_sigbit) & mask;
    masked_val
}

// FIXME: Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {

    /// The calendar year.
    /// 7 bits
    /// The year is not offset. 2009 is 2009.
    fn year(&self) -> usize {
	truncate_bits(self.date.0, 9, 7) as usize
    }

    /// The calendar month, starting at 1 for January. Always in range [1, 12].
    /// 4-bits
    /// January is 1, Feburary is 2, ..., December is 12.
    fn month(&self) -> u8 {
	truncate_bits(self.date.0, 5, 4) as u8
    }

    /// 5-bits
    /// The calendar day, starting at 1. Always in range [1, 31].
    fn day(&self) -> u8 {
	truncate_bits(self.date.0, 0, 5) as u8
    }

    /// 4-bits
    /// The 24-hour hour. Always in range [0, 24).
    fn hour(&self) -> u8 {
	truncate_bits(self.date.0, 12, 4) as u8
    }

    /// 6-bits
    /// The minute. Always in range [0, 60).
    fn minute(&self) -> u8 {
	truncate_bits(self.date.0, 6, 6) as u8
    }

    /// 6-bits
    /// The second. Always in range [0, 60).
    fn second(&self) -> u8 {
	truncate_bits(self.date.0, 0, 6) as u8
    }
}

// FIXME: Implement `traits::Metadata` for `Metadata`.

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_truncator() {
	let val_1: u16 = 0b1111111111111111;

	assert_eq!(truncate_bits(val_1, 0, 16), 0b1111111111111111);
	assert_eq!(truncate_bits(val_1, 0, 15), 0b111111111111111);
	assert_eq!(truncate_bits(val_1, 0, 14), 0b11111111111111);
	assert_eq!(truncate_bits(val_1, 0, 13), 0b1111111111111);
	assert_eq!(truncate_bits(val_1, 0, 12), 0b111111111111);
	assert_eq!(truncate_bits(val_1, 0, 11), 0b11111111111);
	assert_eq!(truncate_bits(val_1, 0, 10), 0b1111111111);
	assert_eq!(truncate_bits(val_1, 0, 9), 0b111111111);
	assert_eq!(truncate_bits(val_1, 0, 8), 0b11111111);
	assert_eq!(truncate_bits(val_1, 0, 7), 0b1111111);
	assert_eq!(truncate_bits(val_1, 0, 6), 0b111111);
	assert_eq!(truncate_bits(val_1, 0, 5), 0b11111);
	assert_eq!(truncate_bits(val_1, 0, 4), 0b1111);
	assert_eq!(truncate_bits(val_1, 0, 3), 0b111);
	assert_eq!(truncate_bits(val_1, 0, 2), 0b11);
	assert_eq!(truncate_bits(val_1, 0, 1), 0b1);

	assert_eq!(truncate_bits(val_1, 1, 15), 0b111111111111111);
	assert_eq!(truncate_bits(val_1, 2, 14), 0b11111111111111);
	assert_eq!(truncate_bits(val_1, 4, 12), 0b111111111111);
	assert_eq!(truncate_bits(val_1, 8, 8), 0b11111111);

	assert_eq!(truncate_bits(0b1000101010101110, 11, 5), 0b10001);
	assert_eq!(truncate_bits(0b1000101010101110, 4, 6), 0b101010);;
	
	
    }
}
