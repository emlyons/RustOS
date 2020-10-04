use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use::core::marker::PhantomData;
use::core::mem::{size_of, transmute};

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub start_cluster: Cluster,
    pub metadata: Metadata,
    pub name: String,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    file_name: [u8; 8],
    file_extension: [u8; 3],
    metadata: Metadata
}

impl VFatRegularDirEntry {
    fn name(&self) -> String {
	let mut name: Vec<u8> = Vec::new();
	name.extend_from_slice(&self.file_name);
	if (self.file_extension[0] != 0x00 || self.file_extension[0] != 0x20) {
	    name.push('.' as u8);
	    name.extend_from_slice(&self.file_extension);
	}
	let mut name_string = String::from_utf8(name).unwrap();
	
	// check for termination characters
	if let Some(term_index) = name_string.find(0x00 as char){
	    name_string.truncate(term_index);
	}
	if let Some(term_index) = name_string.find(0x20 as char){
	    name_string.truncate(term_index);
	}
	return name_string;
    }
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    name_chars: [u16; 5],
    attributes: Attributes,
    entry_type: u8,
    checksum: u8,
    name_chars_second: [u16; 6],
    reserved: [u8; 2],
    name_chars_third: [u16; 2],
}

impl VFatLfnDirEntry {
    fn name(&self) -> String {
	let mut name: Vec<u16> = Vec::new();
	name.extend_from_slice(&self.name_chars);
	name.extend_from_slice(&self.name_chars_second);
	name.extend_from_slice(&self.name_chars_third);
	assert_eq!(name.len(), 10 + 12 + 4);
	let mut name_string = String::from_utf16(&name).unwrap();

	// check for termination characters
	if let Some(term_index) = name_string.find(0x00 as char){
	    name_string.truncate(term_index);
	}
	if let Some(term_index) = name_string.find(0xFF as char){
	    name_string.truncate(term_index);
	}
	return name_string;
    }
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    status: u8,
    _res1: [u8; 10],
    attributes: Attributes,
    _res2: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
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

pub struct DirIterator<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    entries: Vec::<VFatDirEntry>,
    entry_offset: usize,
    // TODO: fields of iterator
}

impl <HANDLE: VFatHandle> DirIterator<HANDLE> {
    fn parse_lfn(&self) -> Option<Entry<HANDLE>> {
	let mut entry: &VFatLfnDirEntry = unsafe {
		&self.entries[self.entry_offset].long_filename
	};
	None
    }

    fn parse_reg(&self) -> Option<Entry<HANDLE>> {
	let mut entry: &VFatRegularDirEntry = unsafe {
		&self.entries[self.entry_offset].regular
	};
	let name = entry.name();

	let x = entry.metadata.attributes;
	
	if entry.metadata.attributes.directory() {
	    //Let Entry::_Dir()
	//}
	//else
	//{
	    //Let Entry::_File()
	}
	None
    }
}

impl <HANDLE: VFatHandle> Iterator for DirIterator<HANDLE> {
    type Item = Entry<HANDLE>;

    
    
    fn next(&mut self) -> Option<Self::Item> {
	// end of directory
	while (self.entry_offset < self.entries.len()) {

	    // determine type of entry
	    let mut unknown_entry: &VFatUnknownDirEntry = unsafe {
		&self.entries[self.entry_offset].unknown
	    };
	    // attempt to parse entry
	    if let Some(entry) = {
		if unknown_entry.attributes.lfn() {
		    self.parse_lfn()
		} else {
		    self.parse_reg()
		}
	    } {
		// return parsed entry or continue to next entry...
		return Some(entry);
	    }	 
	}
	return None;
	// while LFN
	// cast to entry: VFatLfnDirEntry
	// if too small resize file_name to seq_num * (26 bytes/13 UCS-2 char)
	// entry_index = (seq_num - 1)*26
	// file_name.insert_str(entry_index, &entry.name());

	
	// self.entry_offset += 1;
	// entry = self.entries[self.entry_offset];
	
	// END while
	
	// Regular Directory Entry
	// cast to VFatRegularDirEntry

	// CREATE Entry(_File(File<HANDLE>)
	//     or Entry(_Fir(File<HANDLE>)
	
	
    }
}

impl <HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    /// The type of entry stored in this directory.
    type Entry = Entry<HANDLE>;

    /// A type that is an iterator over the entries in this directory.
    type Iter = DirIterator<HANDLE>;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {

	// read in all of directory
	let mut raw: Vec<u8> = Vec::new();
	let size = self.vfat.lock(|v| v.read_chain(Cluster::from(self.start_cluster), &mut raw))?;

	// unsafe cast to Vec::<VFatDirEntry>
	let num_entries = raw.len() / size_of::<VFatDirEntry>();
	let mut entries: Vec::<VFatDirEntry> = unsafe {
	    transmute(raw)
	};
	unsafe {
	    entries.set_len(num_entries);
	}
	
	Ok(DirIterator::<HANDLE>{ phantom: PhantomData, entries: entries, entry_offset: 0})
    }
}
