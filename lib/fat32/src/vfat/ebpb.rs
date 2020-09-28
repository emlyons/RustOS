use core::fmt;
use shim::const_assert_size;
use core::mem::{size_of, transmute};

use crate::traits::BlockDevice;
use crate::vfat::Error;

const EBPB_SIZE: usize = size_of::<BiosParameterBlock>();
const VALID_SIG_A: u8 = 0x28;
const VALID_SIG_B: u8 = 0x29;
const BOOT_SIG: u16 = 0xAA55;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    // FIXME: Fill me in.
    jmp_short_xx_nop: [u8; 3],
    oem_ID: u64,
    byte_per_sector: u16,
    sector_per_cluster: u8,
    reserved_sector: u16,
    num_FAT: u8,
    max_dir_entry: u16,
    total_logical_sector: u16,
    FAT_ID: u8,
    sector_per_FAT: u16,
    sector_per_track: u16,
    num_heads: u16,
    num_hidden_sector: u32,
    total_logical_sector_alt: u32,

    // EBPB
    sector_per_FAT_alt: u32,
    flags: u16,
    FAT_version: u16,
    root_cluster: u32,
    FSInfo: u16,
    backup_boot: u16,
    reserved: [u8; 12],
    drive_number: u8,
    winNT_flags: u8,
    signature: u8,
    volume_ID: u32,
    volume_label: [u8; 11],
    system_ID: [u8; 8],
    boot_code: [u8; 420],
    boot_signature: u16,
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
	let mut sector_data: [u8; EBPB_SIZE] = [0u8; EBPB_SIZE];

	// read sector
	let read_size = device.read_sector(sector, &mut sector_data)?;

	// cast sector to struct BiosParameterBlock
	assert_eq!(read_size, EBPB_SIZE);
	let ebpb = unsafe {
	    transmute::<[u8; EBPB_SIZE], BiosParameterBlock>(sector_data)
	};

	// check signatures
	if ebpb.signature != VALID_SIG_A && ebpb.signature != VALID_SIG_B {
	    return Err(Error::BadSignature);
	}

	if ebpb.boot_signature != BOOT_SIG {
	    return Err(Error::BadSignature);
	}
	
	Ok(ebpb)
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterblock")
            .field("jmp_short_xx_nop", &self.jmp_short_xx_nop)
	    .field("oem_ID", &self.oem_ID)
            .field("byte_per_sector", &self.byte_per_sector)
	    .field("sector_per_cluster", &self.sector_per_cluster)
            .field("reserved_sector", &self.reserved_sector)
	    .field("num_FAT", &self.num_FAT)
            .field("max_dir_entry", &self.max_dir_entry)
	    .field("total_logical_sector", &self.total_logical_sector)
            .field("FAT_ID", &self.FAT_ID)
	    .field("max_dir_entry", &self.max_dir_entry)
            .field("total_logical_sector", &self.total_logical_sector)
	    .field("FAT_ID", &self.FAT_ID)
            .field("sector_per_FAT", &self.sector_per_FAT)
	    .field("sector_per_track", &self.sector_per_track)
            .field("num_heads", &self.num_heads)
	    .field("num_hidden_sector", &self.num_hidden_sector)
	    .field("total_logical_sector_alt", &self.total_logical_sector_alt)
	    .field("sector_per_FAT_alt", &self.sector_per_FAT_alt)
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
	    //.field("boot_code", &self.boot_code)
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

	let mut mock_ebpb_sector = [0u8; 1024];

	// signature
	mock_ebpb_sector[512 + 66] = 0x28;

	// boot signature
	mock_ebpb_sector[512 + 510] = 0x55;
	mock_ebpb_sector[512 + 511] = 0xAA;
	
	let block_device = Cursor::new(&mut mock_ebpb_sector[..]);

	BiosParameterBlock::from(block_device, 1).expect("mock EBPB parse failed");

	Ok(())
    }
}
