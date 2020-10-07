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
    pub cluster: Cluster,
    pub size: u32,
    pub metadata: Metadata,
    pub short_name: String,
    pub long_name: String,
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
	use traits::{Dir, Entry};
	let lowercase_name = {
	    match name.as_ref().to_str() {
		Some(name) => name.to_lowercase(),
		None => {return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid name"))},
	    }
	};
	for entry in self.entries()? {
	    if entry.name().to_lowercase() == lowercase_name {
		return Ok(entry);
	    }
	}
	Err(io::Error::new(io::ErrorKind::NotFound, "entry not found"))
    }

    /// Returns the name of the current directory
    pub fn name(&self) -> &str {
	if self.long_name.is_empty() {
	    &self.short_name
	}
	else {
	    &self.long_name
	}
    }
}

pub struct DirIterator<HANDLE: VFatHandle> {
    vfat: HANDLE,
    entries: Vec::<VFatDirEntry>,
    entry_offset: usize,
}

impl <HANDLE: VFatHandle> DirIterator<HANDLE> {
    /// Parses a long file name entry sequence
    /// Iterates on all LFN entries and builds long file name as well as the regular directory entry
    /// Returns the associated type (File or Directory)
    fn parse_lfn(&mut self) -> Option<Entry<HANDLE>> {

	let mut vec_name: Vec<String> = Vec::new();

	// iterate through all LFN entries
	while (unsafe {self.entries[self.entry_offset].unknown.attributes.lfn()}) {
	    let mut lfn_entry: &VFatLfnDirEntry = unsafe {&self.entries[self.entry_offset].long_filename};

	    // sequence: 0 ... 19
	    let seq_num: usize = ((lfn_entry.sequence_number & 0x1F) - 1) as usize;
	    assert!(seq_num < 20);

	    // extend vec_name to hold all lfn entries
	    if seq_num >= vec_name.len() {
		vec_name.resize(seq_num + 1, String::from(""));
	    }

	    vec_name.insert(seq_num, lfn_entry.name());

	    // go to next entry
	    self.entry_offset += 1;
	}

	self.parse_reg(vec_name.join(""))
    }

    /// Parses a regular directory entry and returns the associated type (File or Directory)
    fn parse_reg(&mut self, long_name: String) -> Option<Entry<HANDLE>> {
	use traits::Metadata;
	
	let mut entry: &VFatRegularDirEntry = unsafe {
		&self.entries[self.entry_offset].regular
	};

	// end of directory
	if entry.file_name[0] == 0x00 {
	    self.entry_offset = self.entries.len();
	    return None;
	}
    
	// increment iterator
	self.entry_offset += 1;

	// deleted entry
	if (entry.file_name[0] == 0xE5 || entry.file_name[0] == 0x00) {
	    return None;
	}

	let name = entry.name();
	
	if entry.metadata.attributes.directory() {
	    let dir_entry = Entry::_Dir(Dir {
	        vfat: self.vfat.clone(),
		cluster: Cluster::from(entry.metadata.cluster()),
		size: entry.metadata.file_size(),
		metadata: entry.metadata,
		short_name: entry.name(),
		long_name: long_name,
	    });
	    return Some(dir_entry);
	}
	else {
	    let file_entry = Entry::_File(File {
	        vfat: self.vfat.clone(),
		cluster: Cluster::from(entry.metadata.cluster()),
		current_cluster: Cluster::from(entry.metadata.cluster()),
		position: 0,
		size: entry.metadata.file_size(),
		metadata: entry.metadata,
		short_name: entry.name(),
		long_name: long_name,
	    });
	    return Some(file_entry);
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
		    self.parse_reg(String::from(""))
		}
	    } {
		// return parsed entry or continue to next entry...
		return Some(entry);
	    }	 
	}
	return None;
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
	let size = self.vfat.lock(|v| v.read_chain(self.cluster, &mut raw))?;

	// unsafe cast to Vec::<VFatDirEntry>
	let num_entries = raw.len() / size_of::<VFatDirEntry>();
	let mut entries: Vec::<VFatDirEntry> = unsafe {
	    transmute(raw)
	};
	unsafe {
	    entries.set_len(num_entries);
	}
	
	Ok(DirIterator::<HANDLE>{ vfat: self.vfat.clone(), entries: entries, entry_offset: 0})
    }
}
