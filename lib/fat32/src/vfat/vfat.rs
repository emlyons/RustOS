use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

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
    fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
	unimplemented!("VFat::read_cluster()")
    }
    
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
	// Data Clusters start >= 2
	// entry 0 = ID, 1 = end of chain (EOC) marker

	// FAT ENTRY
	// FAT_base_addr 
	// entry_size: 4
	// offset: start * entry_size
	// get entry at FAT_base_addr + offset

	// CLUSTER
	// 

	unimplemented!("VFat::read_chain()")
    }
    
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
	unimplemented!("VFat::fat_entry()")
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
