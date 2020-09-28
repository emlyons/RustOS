use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8, 
    sector_cylinder: [u8; 2], // [sector (bits 0:5), cylinder (bits 6:15)]
}

// FIXME: implement Debug for CHS
//impl fmt::Debug for CHS {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        f.debug_struct("CHS")
//            .field("head", &self.head)
//	    .field("sector", &(self.sector_cylinder & 0x003f))
//            .finish("cylinder", &(self.sector_cylinder & 0xFFC0))
//    }
//}

assert_eq!(align_of_vale(u16), 2);
const_assert_size!(CHS, 3);

#[repr(C, packed)]
pub struct PartitionEntry {
    boot_indicator: u8,
    start_chs: CHS,
    partition_type: u8,
    end_chs: CHS,
    relative_sector: u32, // offset, in sectors, from start of disk to start of parition
    total_sectors: u32,
}

// FIXME: implement Debug for PartitionEntry
impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PartitionEntry")
            .field("boot_indicator", &self.boot_indicator)
	    .field("start_chs", &self.start_chs)
	    .field("partition_type", &self.partition_type)
	    .field("end_chs", &self.end_chs)
	    .field("relative_sector", &self.relative_sector)
	    .field("total_sectors", &self.total_sectors)
            .finish()
    }
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    MBR_Bootstrap: [u8; 436],
    disk_ID: [u8; 10],
    pte_first: PartitionEntry,
    pte_second: PartitionEntry,
    pte_third: PartitionEntry,
    pte_fourth: PartitionEntry,
    signature: u16,
}

// FIXME: implemente Debug for MaterBootRecord
impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("disk_ID", &self.disk_ID)
	    .field("pte_first", &self.pte_first)
	    .field("pte_second", &self.pte_second)
	    .field("pte_third", &self.pte_third)
	    .field("pte_fourth", &self.pte_fourth)
	    .field("signature", &self.signature)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
	let sector = [0u8; BlockDevice.sector_size()];
//	let all_sectors = device.read_all_sector(
	//      unimplemented!("MasterBootRecord::from()")
	return Err(Error::BadSignature);
    }
}
