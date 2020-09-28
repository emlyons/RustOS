use core::fmt;
use core::mem::{size_of, transmute};
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

const MBR_SECTOR: u64 = 0;
const MBR_SIZE: usize = size_of::<MasterBootRecord>();
const VALID_BOOTSEC: u16 = 0x55AA;
const INACTIVE_PART: u8 = 0x00;
const ACTIVE_PART: u8 = 0x80;   	

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8, 
    sector_cylinder: [u8; 2], // [sector (bits 0:5), cylinder (bits 6:15)]
}

// FIXME: implement Debug for CHS
impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CHS")
            .field("head", &self.head)
	    .field("sector_cylinder", &self.sector_cylinder[0])
	    .finish()
    }
}

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

/// Verifies the boot indicator of a partition entry conforms to a valid FAT32 value
fn verify_boot_indicator(pte: &PartitionEntry) -> Result<(), Error> {	
    let boot_indicator = pte.boot_indicator;
    
    match boot_indicator == INACTIVE_PART || boot_indicator == ACTIVE_PART {
	true => Ok(()),
	false => Err(Error::UnknownBootIndicator(boot_indicator)),
    }	
}

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
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
	let mut sector_data: [u8; MBR_SIZE] = [0u8; MBR_SIZE];

	// read sector
	let read_size = device.read_sector(MBR_SECTOR, &mut sector_data)?;

	// cast sector_data to struct MasterBootRecord
	if read_size != MBR_SIZE {
	    return Err(Error::Io(io::Error::new(io::ErrorKind::Other, "MasterBootRecord size is invalid")));
	}	
	let mbr = unsafe {
	    transmute::<[u8; MBR_SIZE], MasterBootRecord>(sector_data)
	};

	// check signature
	if mbr.signature != VALID_BOOTSEC {  // on fail return BadSignature
	    return Err(Error::BadSignature);
	}

	// check boot indicators for each pte (i.e. must be 0x00 (inactive) or 0x80 (bootable))
	verify_boot_indicator(&mbr.pte_first)?;
	verify_boot_indicator(&mbr.pte_second)?;
	verify_boot_indicator(&mbr.pte_third)?;
	verify_boot_indicator(&mbr.pte_fourth)?;

	Ok(mbr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mbr_mock_parse() -> Result<(), String> {
	use shim::io::Cursor;

	let mut mock_mbr_sector = [0u8; 512];

	// set "Valid bootsector" signature
	mock_mbr_sector[510] = 0xAA;
	mock_mbr_sector[511] = 0x55;

	// PTE signatures
	mock_mbr_sector[446] = 0x80;
	mock_mbr_sector[462] = 0x00;
	mock_mbr_sector[478] = 0x00;
	mock_mbr_sector[494] = 0x00;
	
	let block_device = Cursor::new(&mut mock_mbr_sector[..]);

	MasterBootRecord::from(block_device).expect("mock MBR parse failed");

	Ok(())
    }
}
