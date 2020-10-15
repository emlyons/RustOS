use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use hashbrown::HashMap;
use shim::io;
use core::cmp;

use crate::traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// Number of sectors
    pub num_sectors: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
    partition: Partition,
}

impl CachedPartition {
    /// Creates a new `CachedPartition` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `0` will be
    /// translated to physical sector `partition.start`. Virtual sectors of
    /// sector number `[0, num_sectors)` are accessible.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedPartition
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());

        CachedPartition {
            device: Box::new(device),
            cache: HashMap::new(),
            partition: partition,
        }
    }

    /// Returns the number of physical sectors that corresponds to
    /// one logical sector.
    fn factor(&self) -> u64 {
        self.partition.sector_size / self.device.sector_size()
    }

    /// Maps a user's request for a sector `virt` to the physical sector.
    /// Returns `None` if the virtual sector number is out of range.
    fn virtual_to_physical(&self, virt: u64) -> Option<u64> {
        if virt >= self.partition.num_sectors {
            return None;
        }

        let physical_offset = virt * self.factor();
        let physical_sector = self.partition.start + physical_offset;
        Some(physical_sector)
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        self.get(sector)?;
	let entry = self.cache.get_mut(&sector).unwrap();
	entry.dirty = true;
	Ok(&mut entry.data)
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        if !self.cache.contains_key(&sector) {
	    let physical_sector = self.virtual_to_physical(sector).expect("attempted to cache invalid sector");	    
	    let num_physical = self.factor();
	    let logical_size: usize = self.partition.sector_size as usize;
	    let physical_size = self.device.sector_size();

	    let mut data = vec![0u8; logical_size];
	    for n in 0..num_physical {
		self.device.read_sector(
		    physical_sector + n,
		    &mut data[(physical_size * n) as usize..],
		)?;
	    }
	    self.cache.insert(sector, CacheEntry {
		data: data,
		dirty: false,
	    });
	}
	Ok(&self.cache[&sector].data)
    }
}

// FIXME: Implement `BlockDevice` for `CacheDevice`. The `read_sector` and
// `write_sector` methods should only read/write from/to cached sectors.
impl BlockDevice for CachedPartition {
    fn sector_size(&self) -> u64 {
	self.partition.sector_size
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        if self.cache.contains_key(&sector) {
	    let entry = &self.cache[&sector].data;
	    let bytes = cmp::min(buf.len(), entry.len());
	    buf[0..bytes].copy_from_slice(&entry[0..bytes]);
	    Ok(bytes)
	}
	else {
	    Err(io::Error::new(io::ErrorKind::Other, "read sector requested not in cache"))
	}
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        unimplemented!()
    }
}

impl fmt::Debug for CachedPartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedPartition")
            .field("device", &"<block device>")
            .field("cache", &self.cache)
            .finish()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use shim::io::Cursor;
    use crate::mbr::MasterBootRecord;
    use crate::vfat::ebpb::BiosParameterBlock;

    static mut data: [u8; 512*10] = [0; 512*10];

    #[test]
    fn test_cache() -> Result<(), String> {
	
	unsafe {
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
	    data[ebpb_start+19] = 0xFF;
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

	    // logical sector to physical sectors(~:0 0:1,2 1:3,4 2:5,6 3:7,8 4:9,10)
	    data[512*7 + 100] = 0x33;
	    data[512*8 + 133] = 0x42;
	    

	    let block_device = Cursor::new(&mut data[..]);
	
	    let mut data_copy: [u8; 512*10] = [0u8; 512*10];
	    data_copy.copy_from_slice(&data);
	    let block_copy = Cursor::new(&mut data_copy[..]);	    
	    let mbr = MasterBootRecord::from(block_copy).expect("mock MBR parse failed");
	    assert_eq!(mbr.first_pte().start_sector(), 0x01);
	    assert_eq!(mbr.first_pte().num_sectors(), 0xFE);

	    let mut data_copy2: [u8; 512*10] = [0u8; 512*10];
	    data_copy2.copy_from_slice(&data);
	    let block_copy2 = Cursor::new(&mut data_copy2[..]);	    
	    let ebpb = BiosParameterBlock::from(block_copy2, mbr.first_pte().start_sector() as u64).expect("mock EBPB parse failed");
	    assert_eq!(ebpb.logical_sector_size(), 1024);
	    assert_eq!(ebpb.logical_per_cluster(), 0x02);
	    assert_eq!(ebpb.fat_start(), 0x01);
	    assert_eq!(ebpb.num_fats(), 0x01);
	    assert_eq!(ebpb.num_logical_sectors(), 0xFF);
	    assert_eq!(ebpb.num_sectors_per_fat(), 0x1);
	    
	    let partition = Partition {
		start: mbr.first_pte().start_sector() as u64,
		num_sectors: mbr.first_pte().num_sectors() as u64,
		sector_size: ebpb.logical_sector_size() as u64,
	    };
	    
	    let mut cache = CachedPartition::new(block_device, partition);

	    let mut buf: [u8; 1024] = [0u8; 1024];
	    if let Ok(_) = cache.read_sector(3, &mut buf) {
		panic!("\n\nread uncached sector\n\n")
	    }

	    let result = cache.get(3).expect("failed to read");
	    assert_eq!(result[100], 0x33);
	    assert_eq!(result[645], 0x42);

	    if let Err(_) = cache.read_sector(3, &mut buf) {
		panic!("\n\nread cached sector\n\n")
	    }
	}
	Ok(())
    }
}
