use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    // FIXME: Fill me in.
    pub metadata: Metadata,
    pub name: String,
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl <HANDLE:VFatHandle> traits::File for File<HANDLE> {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
	unimplemented!("FILE.sync()")
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
	unimplemented!("FILE.size()")
    }
}

impl <HANDLE:VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
	// TODO:
        panic!("Dummy")
    }
    fn flush(&mut self) -> io::Result<()> {
	// TODO:
        panic!("Dummy")
    }
}

impl <HANDLE:VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
	// TODO:
        panic!("Dummy")
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
        unimplemented!("File::seek()")
    }
}
