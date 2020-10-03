use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    // FIXME: Fill me in.
    pub first_cluster: Cluster,
    pub metadata: Metadata,
    pub name: String,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    file_name: [u8; 8],
    file_extension: [u8; 3],
    attributes: u8,
    reserved: u8,
    create_time_tenths: u8,
    create_time: u16,
    create_date: u16,
    access_date: u16,
    cluster_high: u16,
    mod_time: u16,
    mod_date: u16,
    cluster_low: u16,
    file_size: u32,
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    name_characters: [u8; 10],
    attributes: u8,
    entry_type: u8,
    checksum: u8,
    name_characters_second: [u8; 12],
    reserved: [u8; 2],
    name_characters_third: [u8; 4],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    // FIXME: Fill me in.
}

//const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl <HANDLE: VFatHandle> traits::Dir for Dir <HANDLE> {
    /// The type of entry stored in this directory.
    type Entry = traits::Dummy;

    /// A type that is an iterator over the entries in this directory.
    type Iter = traits::Dummy;//Iterator<Item = Self::Entry>;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {
	Err(io::Error::new(io::ErrorKind::Interrupted, "invalid cluster requested"))
    }
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        unimplemented!("Dir::find()")
    }
}

// DEBUG impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    // FIXME: Implement `trait::Dir` for `Dir`.
//    unimplemented!("VFatHandle")
//}
