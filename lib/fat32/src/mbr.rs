use core::fmt;
use core::mem::{size_of, transmute};
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

const MBR_SECTOR: u64 = 0;
const MBR_SIZE: usize = size_of::<MasterBootRecord>();
const VALID_SIGNATURE: u16 = 0xAA55;
const INACTIVE_PARTITION: u8 = 0x00;
const ACTIVE_PARTITION: u8 = 0x80;
const FAT32_ID_1: u8 = 0x0B;
const FAT32_ID_2: u8 = 0x0C;

#[repr(C, packed)]
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
#[derive(Copy, Clone)]
pub struct PartitionEntry {
    boot_indicator: u8,
    start_chs: CHS,
    partition_type: u8,
    end_chs: CHS,
    relative_sector: [u8; 4], // offset, in sectors, from start of disk to start of parition
    total_sectors: [u8; 4],
}

impl PartitionEntry {
    pub fn bootable(&self) -> Result<bool, Error> {
	if self.boot_indicator == ACTIVE_PARTITION {
	    Ok(true)
	}
	else if self.boot_indicator == INACTIVE_PARTITION {
	    Ok(false)
	}
	else {
	    Err(Error::Io(io::Error::new(io::ErrorKind::Other, "pte has an invalid boot indicator")))
	}
    }

    pub fn partition_type(&self) -> bool {
	if self.partition_type == FAT32_ID_1 || self.partition_type == FAT32_ID_2 {
	    true
	}
	else {
	    false
	}   
    }

    // TODO
    pub fn start_sector(&self) -> u32 {
	u32::from_le_bytes(self.relative_sector)
    }

    pub fn num_sectors(&self) -> u32 {
	u32::from_le_bytes(self.total_sectors)
    }
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
#[derive(Copy, Clone)]
pub struct MasterBootRecord {
    MBR_Bootstrap: [u8; 436],
    disk_ID: [u8; 10],
    pte_first: PartitionEntry,
    pte_second: PartitionEntry,
    pte_third: PartitionEntry,
    pte_fourth: PartitionEntry,
    signature: [u8; 2],
}

impl MasterBootRecord {
    pub fn first_pte(&self) -> PartitionEntry {
	self.pte_first
    }

    pub fn second_pte(&self) -> PartitionEntry {
	self.pte_second
    }

    pub fn third_pte(&self) -> PartitionEntry {
	self.pte_third
    }

    pub fn fourth_pte(&self) -> PartitionEntry {
	self.pte_fourth
    }

    pub fn signature(&self) -> bool {
	if u16::from_le_bytes(self.signature) == VALID_SIGNATURE {
	    true
	}
	else {
	    false
	}
    }
}

// FIXME: implemente Debug for MasterBootRecord
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
	let mut data: [u8; MBR_SIZE] = [0u8; MBR_SIZE];

	// read sector
	let read_size = device.read_sector(MBR_SECTOR, &mut data)?;

	// cast sector_data to struct MasterBootRecord
	if read_size != MBR_SIZE {
	    return Err(Error::Io(io::Error::new(io::ErrorKind::Other, "MasterBootRecord size is invalid")));
	}

	let mbr_ptr = data.as_ptr() as *const MasterBootRecord;
	let mbr: MasterBootRecord = unsafe {
	    *mbr_ptr
	};

	//check signature
	if !mbr.signature() {
	    return Err(Error::BadSignature);
	}

	// check boot indicators for each pte (i.e. must be 0x00 (inactive) or 0x80 (bootable))
	if let Err(err) = mbr.first_pte().bootable() {
	    return Err(Error::UnknownBootIndicator(0));
	}
	if let Err(err) = mbr.second_pte().bootable() {
	    return Err(Error::UnknownBootIndicator(1));
	}
	if let Err(err) = mbr.third_pte().bootable() {
	    return Err(Error::UnknownBootIndicator(2));
	}
	if let Err(err) = mbr.fourth_pte().bootable() {
	    return Err(Error::UnknownBootIndicator(3));
	}

	// verify partition type
	if !mbr.first_pte().partition_type() || !mbr.second_pte().partition_type() || !mbr.third_pte().partition_type() || !mbr.fourth_pte().partition_type() {
	    return Err(Error::Io(io::Error::new(io::ErrorKind::Other, "invalid partition type found")));
	}
	
	Ok(mbr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shim::io::Cursor;

    #[test]
    fn mbr_mock_parse() -> Result<(), String> {

	let mut mock_mbr_sector = [0u8; 512];

	// set "Valid bootsector" signature
	mock_mbr_sector[510] = 0x55;
	mock_mbr_sector[511] = 0xAA;

	// PTE signatures
	mock_mbr_sector[446] = 0x80;
	mock_mbr_sector[462] = 0x00;
	mock_mbr_sector[478] = 0x00;
	mock_mbr_sector[494] = 0x00;


	// PTE types
	mock_mbr_sector[450] = FAT32_ID_1;
	mock_mbr_sector[466] = FAT32_ID_2;
	mock_mbr_sector[482] = FAT32_ID_1;
	mock_mbr_sector[498] = FAT32_ID_2;

	// first sector of partition
	mock_mbr_sector[454] = 0x00;
	mock_mbr_sector[455] = 0x11;
	mock_mbr_sector[456] = 0x22;
	mock_mbr_sector[457] = 0x33;
	
	mock_mbr_sector[470] = 0x44;
	mock_mbr_sector[471] = 0x55;
	mock_mbr_sector[472] = 0x66;
	mock_mbr_sector[473] = 0x77;
	
	mock_mbr_sector[486] = 0x88;
	mock_mbr_sector[487] = 0x99;
	mock_mbr_sector[488] = 0xAA;
	mock_mbr_sector[489] = 0xBB;
	
	mock_mbr_sector[502] = 0xCC;
	mock_mbr_sector[503] = 0xDD;
	mock_mbr_sector[504] = 0xEE;
	mock_mbr_sector[505] = 0xFF;

	// sectors in partition
	mock_mbr_sector[458] = 0x00;
	mock_mbr_sector[459] = 0x11;
	mock_mbr_sector[460] = 0x22;
	mock_mbr_sector[461] = 0x33;
	
	mock_mbr_sector[474] = 0x44;
	mock_mbr_sector[475] = 0x55;
	mock_mbr_sector[476] = 0x66;
	mock_mbr_sector[477] = 0x77;
	
	mock_mbr_sector[490] = 0x88;
	mock_mbr_sector[491] = 0x99;
	mock_mbr_sector[492] = 0xAA;
	mock_mbr_sector[493] = 0xBB;
	
	mock_mbr_sector[506] = 0xCC;
	mock_mbr_sector[507] = 0xDD;
	mock_mbr_sector[508] = 0xEE;
	mock_mbr_sector[509] = 0xFF;
	
	let block_device = Cursor::new(&mut mock_mbr_sector[..]);

	let mbr = MasterBootRecord::from(block_device).expect("mock MBR parse failed");

	assert_eq!(mbr.first_pte().start_sector(), 0x33221100);
	assert_eq!(mbr.second_pte().start_sector(), 0x77665544);
	assert_eq!(mbr.third_pte().start_sector(), 0xBBAA9988);
	assert_eq!(mbr.fourth_pte().start_sector(), 0xFFEEDDCC);

	assert_eq!(mbr.first_pte().num_sectors(), 0x33221100);
	assert_eq!(mbr.second_pte().num_sectors(), 0x77665544);
	assert_eq!(mbr.third_pte().num_sectors(), 0xBBAA9988);
	assert_eq!(mbr.fourth_pte().num_sectors(), 0xFFEEDDCC);
	

	Ok(())
    }
}
