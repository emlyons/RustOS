use crate::vfat::*;
use core::fmt;

use self::Status::*;

#[derive(Debug, PartialEq)]
pub enum Status {
    /// The FAT entry corresponds to an unused (free) cluster.
    Free,
    /// The FAT entry/cluster is reserved.
    Reserved,
    /// The FAT entry corresponds to a valid data cluster. The next cluster in
    /// the chain is `Cluster`.
    Data(Cluster),
    /// The FAT entry corresponds to a bad (disk failed) cluster.
    Bad,
    /// The FAT entry corresponds to a valid data cluster. The corresponding
    /// cluster is the last in its chain.
    Eoc(u32),
}

#[repr(C, packed)]
pub struct FatEntry(pub u32);

impl FatEntry {
    /// Returns the `Status` of the FAT entry `self`.
    pub fn status(&self) -> Status {
	// 28-bits of FAT entry are used
	let status = self.0 & 0xFFFFFFF;

	if status == 0x00 {
	    return Status::Free;
	}

	if status == 0x01 {
	    return Status::Reserved;
	}

	if 0x02 <= status && status <= 0xFFFFFEF {
	    return Data(Cluster::from(self.0));
	}

	if 0xFFFFFF0 <= status && status <= 0xFFFFFF6 {
	    return Reserved;
	}

	if status == 0xFFFFFF7 {
	    return Bad;
	}

	if 0xFFFFFF8 <= status && status <= 0xFFFFFFF {
	    return Eoc(self.0);
	}

	unreachable!()
    }
}

impl fmt::Debug for FatEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatEntry")
            .field("value", &{ self.0 })
            .field("status", &self.status())
            .finish()
    }
}
