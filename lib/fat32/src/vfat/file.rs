use alloc::string::String;

use shim::io::{self, SeekFrom};
use core::cmp::{max, min};

use crate::traits;
use crate::vfat::{Cluster, Entry, Metadata, VFatHandle};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub cluster: Cluster,
    pub current_cluster: Cluster,
    pub position: u32,
    pub size: u32,
    pub metadata: Metadata,
    pub short_name: String,
    pub long_name: String,
}

impl <HANDLE:VFatHandle> File<HANDLE> {

    pub fn from(entry: Entry<HANDLE>) -> Option<File<HANDLE>> {
	match entry {
	    Entry::_File(file) => Some(file),
	    _ => None,
	}
    }
    
    /// Returns the name of the current file
    pub fn name(&self) -> &str {
	if self.long_name.is_empty() {
	    &self.short_name
	}
	else {
	    &self.long_name
	}
    }
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl <HANDLE:VFatHandle> traits::File for File<HANDLE> {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
	Ok(())
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
	self.size as u64
    }
}

impl <HANDLE:VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
	unimplemented!("read only file system")
    }
    fn flush(&mut self) -> io::Result<()> {
	Ok(())
    }
}

impl <HANDLE:VFatHandle> io::Read for File<HANDLE> {   
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
	use io::Seek;
	let bytes_per_cluster: u32 = self.vfat.lock(|v| v.cluster_size()) as u32;
	let mut bytes_read: usize = 0;
	let bytes_to_read: u32 = min(_buf.len() as u32, (self.size - self.position));

	while (bytes_read as u32) < bytes_to_read {
	    let offset = (self.position % bytes_per_cluster);
	    let bytes_left_in_cluster = bytes_per_cluster - offset;
	    
	    bytes_read += self.vfat.lock(|v| v.read_cluster(self.current_cluster, offset as usize, &mut _buf[bytes_read..]))?;
	    
	    self.seek(SeekFrom::Current(bytes_read as i64));
	}
	Ok(bytes_read as usize)
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
	let mut long_pos: u64 = 0;

	// safely convert to 32 bit (FAT32) file offset
	match _pos {
	    SeekFrom::Start(offset) => {long_pos = offset;},
	    SeekFrom::End(offset) => {long_pos = add_signed_unsigned(self.size as u64, offset);},
	    SeekFrom::Current(offset) => {long_pos = add_signed_unsigned(self.position as u64, offset);},
	}

	if long_pos >= self.size as u64 {
	    return Err(io::Error::new(io::ErrorKind::InvalidInput, "cannot seek after end of file"));
	}
	let pos = long_pos as u32;

	// maintain current cluster
	let bytes_per_cluster = self.vfat.lock(|v| v.cluster_size());
	let start_of_current_cluster = self.position - (self.position % bytes_per_cluster);
	let start_of_next_cluster = self.position + (bytes_per_cluster - (self.position % bytes_per_cluster));
	let end_of_next_cluster = start_of_next_cluster + bytes_per_cluster - 1;
	if start_of_current_cluster <= pos && pos < start_of_next_cluster {
	    // same cluster
	} else if start_of_next_cluster <= pos && pos <= end_of_next_cluster {
	    // if next cluster in sequence, do a fast get
	    self.current_cluster = self.vfat.lock(|v| v.next_cluster(self.current_cluster))?;
	}
	else {
	    // if not, linear lookup of cluster
	    self.current_cluster = self.vfat.lock(|v| v.find_cluster(pos as usize))?;
	}

	// update file byte offset
	self.position = pos;
	Ok(pos as u64)
    }
}

/// returns a + b where b is a signed value.
/// saturates at 0 or u64::MAX
fn add_signed_unsigned(a: u64, b: i64) -> u64 {
    let _b = b.abs() as u64;
    if b >= 0 {
	a.saturating_add(_b)
    }
    else {
	a.saturating_sub(_b)
    }	
}
