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
	let mut mbr = MasterBootRecord::from(&mut device)?;
	let pte = mbr.get_vfat_pte()?;
	let ebpb = BiosParameterBlock::from(&mut device, pte.relative_sector as u64)?;

	let num_logical_sector = match ebpb.total_logical_sector == 0 {
	    true => {ebpb.total_logical_sector_alt as u64},
	    false => {ebpb.total_logical_sector as u64},
	};
		
	let partition = Partition { start: pte.relative_sector as u64, num_sectors: num_logical_sector as u64, sector_size: ebpb.byte_per_sector as u64 };
	
	let vfat: VFat<HANDLE> = VFat {
	    phantom: PhantomData,
	    device: CachedPartition::new(device, partition),
	    bytes_per_sector: ebpb.byte_per_sector,
	    sectors_per_cluster: ebpb.sector_per_cluster,
	    sectors_per_fat: ebpb.sector_per_FAT_alt,
	    fat_start_sector: pte.relative_sector as u64 + ebpb.reserved_sector as u64,
	    data_start_sector: pte.relative_sector as u64 + ebpb.reserved_sector as u64 + ebpb.sector_per_FAT_alt as u64 * ebpb.num_FAT as u64,
	    rootdir_cluster: Cluster::from(ebpb.root_cluster),
	};

	Ok(VFatHandle::new(vfat))	 
    }

    // TODO: The following methods may be useful here:
    //
    //  * A method to read from an offset of a cluster into a buffer.
    //
    pub fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {

	if cluster.number() < 2 {
	    return Err(io::Error::new(io::ErrorKind::Interrupted, "invalid cluster requested"));
	}
		
	let sectors_in_cluster: usize = self.sectors_per_cluster as usize;
	let mut buf_remaining: usize = buf.len();
	let mut buf_loc: usize = 0;
	
	let cluster_start_sector: usize = self.data_start_sector as usize + sectors_in_cluster*(cluster.number() as usize - 2);
	let start_sector: u64 = (cluster_start_sector + (offset / self.bytes_per_sector as usize)) as u64;
	let bound_sector: u64 = (cluster_start_sector + sectors_in_cluster) as u64;

	let mut offset: usize = offset % self.bytes_per_sector as usize;
	
	for sector in start_sector..bound_sector {
	    let sector_data = self.device.get(sector)?;
	    let read_bytes = cmp::min(buf_remaining, self.bytes_per_sector as usize - offset);

	    buf[buf_loc..buf_loc + read_bytes].clone_from_slice(&sector_data[offset..offset + read_bytes]);
	    
	    buf_loc += read_bytes;
	    buf_remaining -= read_bytes;
	    offset = 0;
	}
	Ok(buf_loc)
    }

/*
    pub fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {

	let sectors_in_cluster: usize = self.sectors_per_cluster as usize;
	let buf_remaining: usize = buf.len();
	let buf_loc: usize = 0;
	
	let cluster_start_sector: usize = self.data_start_sector as usize + sectors_in_cluster*(cluster.number() as usize - 2);
	let sector_offset: usize = offset / self.bytes_per_sector as usize;
	let byte_offset: usize = offset % self.bytes_per_sector as usize;
	let start_sector: usize = cluster_start_sector + sector_offset;
	let bound_sector: usize = cluster_start_sector + sectors_in_cluster;

	eprintln!("\n\n start sector: {} \n\n", sector);
	for sector in start_sector..bound_sector {
	    //let sector_data = self.get(sector)?;
	    // read_bytes = min ( buff_remaining , sector_size - sector_offset)
	    // read from [sector_offset..sector_offset + read_bytes] to buf_loc
	
	    // buf_loc += read_bytes
	    // buff_remaining -= read_bytes
	    // offset = 0
	    eprintln!("\n\n sector: {} \n\n", sector);
	}
	panic!();
	Ok(buf_loc)
    }
*/    

    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {

	let cluster = start;
	let FAT_entry = self.fat_entry(cluster);

	loop {
	    // get FAT entry
	    // get Cluster

	    // if FAT entry data FAT32
	
	    // read all clusters into buffer

	    // follow cluster chain	    
	}
	
	unimplemented!("VFat::read_chain()")
    }
    
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
	const entry_size: u64 = size_of::<FatEntry>() as u64; // 32-bit FAT entries	
	let FAT_table: &mut [u8];

	let sector_offset: u64 = entry_size * (cluster.number() as u64) / self.bytes_per_sector as u64;
	let entry_offset: usize = entry_size as usize * (cluster.number() as usize) % self.bytes_per_sector as usize;

	// get FAT table
	let FAT_table = self.device.get(self.fat_start_sector + sector_offset)?;
	
	let entry: &[FatEntry] = unsafe{
	    FAT_table[entry_offset..entry_offset + entry_size as usize].cast()
	};
	
	Ok(&entry[0])
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
    use std::sync::{Arc, Mutex};
    use std::fmt::{self, Debug};

    macro resource($name:expr) {{
	let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ext/fat32-imgs/", $name);
	match ::std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) => {
		eprintln!(
                    "\nfailed to find assignment 2 resource '{}': {}\n\
                     => perhaps you need to run 'make fetch'?",
                    $name, e
		);
		panic!("missing resource");
            }
	}
    }}
    
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

    /*
     SHUFFLE
     */
    struct Shuffle<T: BlockDevice> {
	device: T,
	swap_address: u64,
    }

    // Swap two
    impl<T: BlockDevice> Shuffle<T> {
	fn new(device: T, swap_address: u64) -> Self {
            let sector_size = device.sector_size();
            assert_eq!(
		swap_address / sector_size,
		(swap_address + 63) / sector_size
            );
	    
            Shuffle {
		device,
		swap_address,
            }
	}
	
	fn swap_target_n(&self) -> u64 {
            self.swap_address / self.sector_size()
	}
	

	fn swap_target_offset(&self) -> u64 {
            self.swap_address % self.sector_size()
	}
    }
    
    impl<T: BlockDevice> BlockDevice for Shuffle<T> {
	fn sector_size(&self) -> u64 {
            self.device.sector_size()
	}
	
	fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
            let bytes = self.device.read_sector(n, buf)?;
            if n == self.swap_target_n() {
		let offset = self.swap_target_offset() as usize;
		
		let mut front = [0u8; 32];
		front.copy_from_slice(&buf[offset..offset + 32]);
		let mut rear = [0u8; 32];
		rear.copy_from_slice(&buf[offset + 32..offset + 64]);
		
		buf[offset..offset + 32].copy_from_slice(&rear);
		buf[offset + 32..offset + 64].copy_from_slice(&front);
            }
            Ok(bytes)
	}
	
	fn write_sector(&mut self, n: u64, buf: &[u8]) -> io::Result<usize> {
            let len = self.sector_size() as usize;
            let mut new_buf = vec![0; len];
            let buf = if n == self.swap_target_n() {
		let offset = self.swap_target_offset() as usize;
		
		new_buf.copy_from_slice(&buf[..len]);
		new_buf[offset..offset + 32].copy_from_slice(&buf[offset + 32..offset + 64]);
		new_buf[offset + 32..offset + 64].copy_from_slice(&buf[offset..offset + 32]);
		
		&new_buf
            } else {
		buf
            };
            self.device.write_sector(n, buf)
	}
    }

    
    
    #[test]
    fn mock_read() -> Result<(), String> {

	const buf_size: usize = 1024;
	
	let shuffle = Shuffle::new(resource!("mock1.fat32.img"), 0x896ca0);
	let vfat = VFat::<StdVFatHandle>::from(shuffle).expect("failed to initialize VFAT from image");
	
	let mut buf = [0u8; buf_size];
	for offset in 0..512 {
	    let bytes_read = vfat.lock(|v| v.read_cluster(Cluster::from(2), offset, &mut buf)).expect("failed to read sector");
	    assert_eq!(bytes_read, cmp::min(buf.len(), 512 - offset));
	}
		
	Ok(())
    }

}
