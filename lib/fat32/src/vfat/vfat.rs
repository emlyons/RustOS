use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;
use core::cmp;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
	let mbr = MasterBootRecord::from(&mut device)?;
	let pte = mbr.first_pte();
	let ebpb = BiosParameterBlock::from(&mut device, pte.start_sector() as u64)?;
	
	let partition = Partition {
	    start: ebpb.fat_start() as u64,
	    num_sectors: ebpb.num_logical_sectors() as u64,
	    sector_size: ebpb.logical_sector_size() as u64,
	};
	
	let cache = CachedPartition::new(device, partition);
	
	let vfat: VFat<HANDLE> = VFat {
	    phantom: PhantomData,
	    device: cache,
	    bytes_per_sector: ebpb.logical_sector_size() as u16,
	    sectors_per_cluster: ebpb.logical_per_cluster() as u8,
	    sectors_per_fat: ebpb.num_sectors_per_fat(),
	    fat_start_sector: ebpb.fat_start() as u64,
	    data_start_sector:  ebpb.fat_start() as u64 + ebpb.num_sectors_per_fat() as u64 * ebpb.num_fats() as u64,
	    rootdir_cluster: Cluster::from(ebpb.root_cluster()),
	};

	Ok(VFatHandle::new(vfat))
    }

    //  * A method to read from an offset of a cluster into a buffer.
    //
    fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
	if !cluster.is_valid() {
	    return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid cluster request into FAT table"));
	}
	let bytes_remaining: usize = cmp::min(
	    self.bytes_per_sector as usize * self.sectors_per_cluster as usize - offset,
	    buf.len(),
	);
	let mut sector: u64 = self.data_start_sector + cluster.index() as u64 * self.sectors_per_cluster as u64 + offset as u64 / self.bytes_per_sector as u64;
	
	//sector = self.data_start_sector + offset as u64 / self.bytes_per_sector as u64;
	let mut byte_offset: usize = offset % self.bytes_per_sector as usize;
	let mut bytes_read = 0;
	while bytes_read < bytes_remaining {
	    let data = self.device.get(sector)?;
	    let read_size = cmp::min(self.bytes_per_sector as usize - byte_offset, buf.len() - bytes_read);
	    buf[bytes_read..bytes_read + read_size].copy_from_slice(&data[byte_offset..byte_offset + read_size]);
	    bytes_read += read_size;
	    sector += 1;
	    byte_offset = 0;
	}	
	Ok(bytes_read)
    }

    //
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
	let cluster_size: usize = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
	let mut current_cluster = start;
	let mut bytes_read = 0;
	loop {
	    let entry = self.fat_entry(current_cluster)?;
	    println!("fat entry: {:?}", entry);
	    match entry.status() {
		Status::Data(next_cluster) => {
		    buf.resize(bytes_read + cluster_size, 0);
		    bytes_read += self.read_cluster(current_cluster, 0, &mut buf[bytes_read..])?;// read cluster -> add to buf
		    current_cluster = next_cluster;
		},
		Status::Eoc(_) => {
		    buf.resize(bytes_read + cluster_size, 0);
		    bytes_read += self.read_cluster(current_cluster, 0, &mut buf[bytes_read..])?;// read cluster -> add to buf
		    return Ok(bytes_read);
		},
		Status::Free => {
		    return Err(io::Error::new(io::ErrorKind::InvalidInput, "attempted to read from free cluster"));
		},
		Status::Reserved => {
		    return Err(io::Error::new(io::ErrorKind::InvalidInput, "attempted to read 'reserved' cluster"));
		},
		Status::Bad => {
		    return Err(io::Error::new(io::ErrorKind::InvalidData, "bad cluster could not be read"));
		},
		_ => unreachable!(),
	    }
	}
	
	panic!()
	//return Err(io::Error::new(io::ErrorKind::InvalidInput, "unimplemented"));
    }
    
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
	if !cluster.is_valid() {
	    return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid cluster request into FAT table"));
	}

	let bytes_from_start: usize = cluster.number() as usize * size_of::<FatEntry>() as usize;
	let byte_offset: usize = bytes_from_start % self.bytes_per_sector as usize;
	let sector_offset_into_fat: usize = bytes_from_start / self.bytes_per_sector as usize;
	let fat_sector = self.fat_start_sector as u64 + sector_offset_into_fat as u64;
	let fat_data = self.device.get(fat_sector)?;	
	let fat_entry: &[FatEntry] = unsafe {
	    fat_data.cast()
	};

	Ok(&fat_entry[byte_offset / size_of::<FatEntry>()])
    }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = crate::traits::Dummy;
    type Dir = crate::traits::Dummy;
    type Entry = crate::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use shim::io::Cursor;
    use crate::vfat::VFat;

    use std::sync::{Arc, Mutex};
    use std::fmt::{self, Debug};

    static mut data: [u8; 1024*9] = [0; 1024*9];

    #[derive(Clone)]
    struct StdVFatHandle(Arc<Mutex<VFat<Self>>>);

    impl Debug for StdVFatHandle {
	fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(f, "StdVFatHandle")
	}
    }

    impl VFatHandle for StdVFatHandle {
	fn new(val: VFat<StdVFatHandle>) -> Self {
            StdVFatHandle(Arc::new(Mutex::new(val)))
	}
	
	fn lock<R>(&self, f: impl FnOnce(&mut VFat<StdVFatHandle>) -> R) -> R {
            f(&mut self.0.lock().expect("all okay"))
	}
    }

    fn get_block() -> Cursor<&'static mut[u8]> {
	let block_device = unsafe {
	    // set "Valid bootsector" signature
	    data[510] = 0x55;
	    data[511] = 0xAA;
	    
	    // PTE signatures
	    data[446] = 0x80;
	    data[462] = 0x00;
	    data[478] = 0x00;
	    data[494] = 0x00;
	    
	    
	    // PTE types
	    data[450] = 0x0B;
	    data[466] = 0x0C;
	    data[482] = 0x0C;
	    data[498] = 0x0C;
	    
	    // first sector of partition
	    data[454] = 0x01;
	    data[455] = 0x00;
	    data[456] = 0x00;
	    data[457] = 0x00;
	    
	    data[470] = 0x00;
	    data[471] = 0x00;
	    data[472] = 0x00;
	    data[473] = 0x00;
	    
	    data[486] = 0x00;
	    data[487] = 0x00;
	    data[488] = 0x00;
	    data[489] = 0x00;
	    
	    data[502] = 0x00;
	    data[503] = 0x00;
	    data[504] = 0x00;
	    data[505] = 0x00;
	    
	    // sectors in partition
	    data[458] = 0xFE;
	    data[459] = 0x00;
	    data[460] = 0x00;
	    data[461] = 0x00;
	    
	    data[474] = 0x44;
	    data[475] = 0x55;
	    data[476] = 0x66;
	    data[477] = 0x77;
	    
	    data[490] = 0x88;
	    data[491] = 0x99;
	    data[492] = 0xAA;
	    data[493] = 0xBB;
	    
	    data[506] = 0xCC;
	    data[507] = 0xDD;
	    data[508] = 0xEE;
	    data[509] = 0xFF;
	    
	    let ebpb_start = 512;
	    
	    // bytes per logical sector
	    data[ebpb_start+11] = 0x00;
	    data[ebpb_start+12] = 0x04;

	    // logical sectors per cluster
	    data[ebpb_start+13] = 0x02;
	    
	    // fat start sector offset
	    data[ebpb_start+14] = 0x01;
	    data[ebpb_start+15] = 0x00;
	    
	    // number of FAT copies
	    data[ebpb_start+16] = 0x01;
	    
	    // sectors on partition
	    data[ebpb_start+19] = 0x7F;
	    data[ebpb_start+20] = 0;
	    
	    data[ebpb_start+32] = 0;
	    data[ebpb_start+33] = 0;
	    data[ebpb_start+34] = 0;
	    data[ebpb_start+35] = 0;
	    
	    // sectors per FAT
	    data[ebpb_start+22] = 0;
	    data[ebpb_start+23] = 0;
	    
	    data[ebpb_start+36] = 0x01;
	    data[ebpb_start+37] = 0;
	    data[ebpb_start+38] = 0;
	    data[ebpb_start+39] = 0;
	    
	    // root cluster
	    data[ebpb_start+44] = 0x02;
	    data[ebpb_start+45] = 0;
	    data[ebpb_start+46] = 0;
	    data[ebpb_start+47] = 0;
	    
	    // signature
	    data[ebpb_start+66] = 0x29;
	    
	    // boot signature
	    data[ebpb_start+510] = 0x55;
	    data[ebpb_start+511] = 0xAA;

	    let fat_start = ebpb_start + 1024;

	    // FAT Entries
	    // entry 0
	    data[fat_start] = 0xFF;
	    data[fat_start + 1] = 0xFF;
	    data[fat_start + 2] = 0xFF;
	    data[fat_start + 3] = 0xFF;

	    // entry 1
	    data[fat_start + 4] = 0xF8;
	    data[fat_start + 5] = 0xFF;
	    data[fat_start + 6] = 0xFF;
	    data[fat_start + 7] = 0xFF;

	    // entry 2
	    data[fat_start + 8] = 0x04;
	    data[fat_start + 9] = 0;
	    data[fat_start + 10] = 0;
	    data[fat_start + 11] = 0;

	    // entry 3
	    data[fat_start + 12] = 0xF8;
	    data[fat_start + 13] = 0xFF;
	    data[fat_start + 14] = 0xFF;
	    data[fat_start + 15] = 0xFF;

	    // entry 4
	    data[fat_start + 16] = 0x03;
	    data[fat_start + 17] = 0;
	    data[fat_start + 18] = 0;
	    data[fat_start + 19] = 0;


	    // DATA
	    let cluster_two = ebpb_start + 2*1024;
	    data[cluster_two..cluster_two+4].copy_from_slice(&[99,2,2,2]);
	    data[cluster_two+100..cluster_two+108].copy_from_slice(&[3,4,5,6,7,8,9,10]);
	    data[cluster_two+1024..cluster_two+1028].copy_from_slice(&[33,2,2,2]);
	    

	    let cluster_three = cluster_two + 2*1024;
	    data[cluster_three..cluster_three+4].copy_from_slice(&[99,3,3,3]);
	    data[cluster_three+1024..cluster_three+1028].copy_from_slice(&[33,3,3,3]);

	    let cluster_four = cluster_three + 2*1024;
	    data[cluster_four..cluster_four+4].copy_from_slice(&[99,4,4,4]);
	    data[cluster_four+1024..cluster_four+1028].copy_from_slice(&[33,4,4,4]);
	    
	    

	    Cursor::new(&mut data[..])
	};
	return block_device;
    }

    #[test]
    fn test_vfat_metadata() -> Result<(), String> {
	let block_device = get_block();

	let vfat = VFat::<StdVFatHandle>::from(block_device).expect("failed to initialize VFAT from image");

	let bytes_per_sector = vfat.lock(|v| v.bytes_per_sector);
	let sectors_per_cluster = vfat.lock(|v| v.sectors_per_cluster);
	let sectors_per_fat = vfat.lock(|v| v.sectors_per_fat);
	let fat_start_sector = vfat.lock(|v| v.fat_start_sector);
	let data_start_sector = vfat.lock(|v| v.data_start_sector);
	let rootdir_cluster = vfat.lock(|v| v.rootdir_cluster);

	assert_eq!(bytes_per_sector, 1024);
	assert_eq!(sectors_per_cluster, 2);
	assert_eq!(sectors_per_fat, 1);
	assert_eq!(fat_start_sector, 1);
	assert_eq!(data_start_sector, 2);
	assert_eq!(rootdir_cluster.number(), 2);

	Ok(())
    }

    #[test]
    fn test_vfat_read_cluster() -> Result<(), String> {
	let block_device = get_block();

	let vfat = VFat::<StdVFatHandle>::from(block_device).expect("failed to initialize VFAT from image");
	let bytes_per_sector = vfat.lock(|v| v.bytes_per_sector) as usize;
	let sectors_per_cluster = vfat.lock(|v| v.sectors_per_cluster) as usize;

	let mut buf = vec![0u8; 2048];
	
	let mut cluster = Cluster::from(2);
	let mut read = vfat.lock(|v| v.read_cluster(cluster, 0, buf.as_mut_slice())).unwrap();
	assert_eq!(buf[0..4], [99,2,2,2]);
	assert_eq!(buf[100..108], [3,4,5,6,7,8,9,10]);
	assert_eq!(buf[1024..1028], [33,2,2,2]);
	assert_eq!(read, bytes_per_sector * sectors_per_cluster);
	
	cluster = Cluster::from(2);
	read = vfat.lock(|v| v.read_cluster(cluster, 100, buf.as_mut_slice())).unwrap();
	assert_eq!(buf[0..8], [3,4,5,6,7,8,9,10]);
	assert_eq!(read, bytes_per_sector * sectors_per_cluster - 100);

	cluster = Cluster::from(3);
	read = vfat.lock(|v| v.read_cluster(cluster, 0, buf.as_mut_slice())).unwrap();
	assert_eq!(buf[0..4], [99,3,3,3]);
	assert_eq!(buf[1024..1028], [33,3,3,3]);
	assert_eq!(read, bytes_per_sector * sectors_per_cluster);

	cluster = Cluster::from(3);
	read = vfat.lock(|v| v.read_cluster(cluster, 1024, buf.as_mut_slice())).unwrap();
	assert_eq!(buf[0..4], [33,3,3,3]);
	assert_eq!(read, bytes_per_sector * sectors_per_cluster - 1024);

	cluster = Cluster::from(4);
	read = vfat.lock(|v| v.read_cluster(cluster, 0, buf.as_mut_slice())).unwrap();
	assert_eq!(buf[0..4], [99,4,4,4]);
	assert_eq!(buf[1024..1028], [33,4,4,4]);
	assert_eq!(read, bytes_per_sector * sectors_per_cluster);
	
	println!("\n\nCLUSTER {}: {:?}\n", cluster.number(), buf);

	Ok(())
    }

    #[test]
    fn test_vfat_read_chain() -> Result<(), String> {
	let block_device = get_block();

	let vfat = VFat::<StdVFatHandle>::from(block_device).expect("failed to initialize VFAT from image");
	let bytes_per_sector = vfat.lock(|v| v.bytes_per_sector) as usize;
	let sectors_per_cluster = vfat.lock(|v| v.sectors_per_cluster) as usize;

	let mut buf: Vec<u8> = Vec::new();
	
	let mut cluster = Cluster::from(2);
	let mut read = vfat.lock(|v| v.read_chain(cluster, &mut buf)).unwrap();

	println!("read_chain() returned");
	
	assert_eq!(buf[0..4], [99,2,2,2]);
	assert_eq!(buf[100..108], [3,4,5,6,7,8,9,10]);
	assert_eq!(buf[1024..1028], [33,2,2,2]);

	assert_eq!(buf[2048..2052], [99,4,4,4]);
	assert_eq!(buf[3072..3076], [33,4,4,4]);

	assert_eq!(buf[4096..4100], [99,3,3,3]);
	assert_eq!(buf[5120..5124], [33,3,3,3]);

	assert_eq!(read, 3 * bytes_per_sector * sectors_per_cluster);
	
	Ok(())
    }
}
