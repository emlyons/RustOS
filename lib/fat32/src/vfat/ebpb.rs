use core::fmt;
use shim::const_assert_size;
use core::mem::{size_of, transmute};

use crate::traits::BlockDevice;
use crate::vfat::Error;

const EBPB_SIZE: usize = size_of::<BiosParameterBlock>();
const VALID_SIG_1: u8 = 0x28;
const VALID_SIG_2: u8 = 0x29;
const BOOT_SIG: u16 = 0xAA55;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct BiosParameterBlock {
    jmp_short_xx_nop: [u8; 3],
    oem_ID: [u8; 8],
    bytes_per_sector: [u8; 2],
    sector_per_cluster: u8,
    reserved_sectors: [u8; 2],
    num_FAT: u8,
    max_dir_entry: [u8; 2],
    total_logical_sectors: [u8; 2],
    FAT_ID: u8,
    sectors_per_FAT: [u8; 2],
    sector_per_track: [u8; 2],
    num_heads: [u8; 2],
    num_hidden_sector: [u8; 4],
    total_logical_sectors_alt: [u8; 4],

    // Extended BPB
    sectors_per_FAT_alt: [u8; 4],
    flags: [u8; 2],
    FAT_version: [u8; 2],
    root_cluster: [u8; 4],
    FSInfo: [u8; 2],
    backup_boot: [u8; 2],
    reserved: [u8; 12],
    drive_number: u8,
    winNT_flags: u8,
    signature: u8,
    volume_ID: [u8; 4],
    volume_label: [u8; 11],
    system_ID: [u8; 8],
    boot_code: [u8; 420],
    boot_signature: [u8; 2],
}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
	let mut data: [u8; EBPB_SIZE] = [0u8; EBPB_SIZE];

	// read sector
	let read_size = device.read_sector(sector, &mut data)?;

	// cast sector to struct BiosParameterBlock
	assert_eq!(read_size, EBPB_SIZE);

	let ebpb_ptr = data.as_ptr() as *const BiosParameterBlock;
	let ebpb = unsafe {
	    *ebpb_ptr
	};

	if !ebpb.boot_signature() {
	    return Err(Error::BadSignature);
	}
	
	Ok(ebpb)
    }

    /// byte size of logical sectors for partition
    pub fn logical_sector_size(&self) -> u32 {
	u16::from_le_bytes(self.bytes_per_sector) as u32
    }

    /// logical sectors per cluster for partition
    pub fn logical_per_cluster(&self) -> u32 {
	self.sector_per_cluster as u32
    }

    /// byte size of a cluster for partition
    pub fn cluster_size(&self) -> u32 {
	self.logical_per_cluster() * self.logical_sector_size()
    }

    /// offset in logical sectors from start of partition (".start_sector()" in PTE) to first data cluster
    pub fn data_start(&self) -> u32 {
	u16::from_le_bytes(self.reserved_sectors) as u32
    }

    /// number of file allocation tables (COPIES) for partition
    pub fn num_fats(&self) -> u32 {
	self.num_FAT as u32
    }

    /// the total number of logical sectors in the partition
    pub fn num_logical_sectors(&self) -> u32 {
	let num = u16::from_le_bytes(self.total_logical_sectors);
	if num > 0 {
	    num as u32
	}
	else {
	    u32::from_le_bytes(self.total_logical_sectors_alt)
	}		
    }

    /// number of sectors used for a FAT
    pub fn num_sectors_per_fat(&self) -> u32 {
	let num = u16::from_le_bytes(self.sectors_per_FAT);
	if num > 0 {
	    num as u32
	}
	else {
	    u32::from_le_bytes(self.sectors_per_FAT_alt)
	}
    }

    /// cluster number where root directory begins
    pub fn root_cluster(&self) -> u32 {
	u32::from_le_bytes(self.root_cluster)
    }

    /// returns true if EBPB signature is valid
    pub fn signature(&self) -> bool {
	if self.signature == VALID_SIG_1 || self.signature == VALID_SIG_2 {
	    true
	}
	else {
	    false
	}
    }

    /// returns true if EBPB boot signature is valid
    pub fn boot_signature(&self) -> bool {
	let boot_signature = u16::from_le_bytes(self.boot_signature);
	if boot_signature ==  BOOT_SIG {
	    true
	}
	else {
	    false
	}
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterblock")
            .field("jmp_short_xx_nop", &self.jmp_short_xx_nop)
	    .field("oem_ID", &self.oem_ID)
            .field("byte_per_sector", &self.bytes_per_sector)
	    .field("sector_per_cluster", &self.sector_per_cluster)
            .field("reserved_sector", &self.reserved_sectors)
	    .field("num_FAT", &self.num_FAT)
            .field("max_dir_entry", &self.max_dir_entry)
	    .field("total_logical_sectors", &self.total_logical_sectors)
            .field("FAT_ID", &self.FAT_ID)
            .field("sector_per_FAT", &self.sectors_per_FAT)
	    .field("sector_per_track", &self.sector_per_track)
            .field("num_heads", &self.num_heads)
	    .field("num_hidden_sector", &self.num_hidden_sector)
	    .field("total_logical_sectors_alt", &self.total_logical_sectors_alt)
	    .field("sector_per_FAT_alt", &self.sectors_per_FAT_alt)
	    .field("flags", &self.flags)
	    .field("FAT_version", &self.FAT_version)
	    .field("root_cluster", &self.root_cluster)
	    .field("FSInfo", &self.FSInfo)
	    .field("backup_boot", &self.backup_boot)
	    .field("reserved", &self.reserved)
	    .field("drive_number", &self.drive_number)
	    .field("winNT_flags", &self.winNT_flags)
	    .field("signature", &self.signature)
	    .field("volume_ID", &self.volume_ID)
	    .field("volume_label", &self.volume_label)
	    .field("system_ID", &self.system_ID)
	    .field("boot_signature", &self.boot_signature)
	    .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebpb_mock_parse() -> Result<(), String> {
	use shim::io::Cursor;

	let mut data = [0u8; 1024];

	// bytes per logical sector
	data[11] = 0xFF;
	data[12] = 0x01;

	// logical sectors per cluster
	data[13] = 0x33;

	// data start sector (first sector of cluster 2)
	data[14] = 0x77;
	data[15] = 0x88;

	// number of FAT copies
	data[16] = 0x02;

	// sectors on partition
	data[19] = 0;
	data[20] = 0;

	data[32] = 0x21;
	data[33] = 0x43;
	data[34] = 0x65;
	data[35] = 0x87;

	// sectors per FAT
	data[22] = 0;
	data[23] = 0;

	data[36] = 0x12;
	data[37] = 0x34;
	data[38] = 0x56;
	data[39] = 0x78;

	// root cluster
	data[44] = 0x02;
	data[45] = 0;
	data[46] = 0;
	data[47] = 0x0C;
	
	
	
	// signature
	data[66] = 0x29;

	// boot signature
	data[510] = 0x55;
	data[511] = 0xAA;
	
	let block_device = Cursor::new(&mut data[..]);

	let ebpb = BiosParameterBlock::from(block_device, 0).expect("mock EBPB parse failed");

	assert_eq!(ebpb.logical_sector_size(), 0x1FF);
	assert_eq!(ebpb.logical_per_cluster(), 0x33);
	assert_eq!(ebpb.data_start(), 0x8877);
	assert_eq!(ebpb.num_fats(), 0x02);
//	assert_eq!(ebpb.num_logical_sectors(), 0x21AA);
	assert_eq!(ebpb.num_logical_sectors(), 0x87654321);
//	assert_eq!(ebpb.num_sectors_per_fat(), 0x3670);
	assert_eq!(ebpb.num_sectors_per_fat(), 0x78563412);

	assert_eq!(ebpb.root_cluster(), 0x0C000002);
	
	Ok(())
    }
}
