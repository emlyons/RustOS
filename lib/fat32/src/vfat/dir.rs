use alloc::string::String;
use alloc::vec::Vec;
use alloc::str;

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
	let mut name = String::from(String::from_utf8_lossy(&self.file_name)); // get short file name
	// truncate at any terminating chars
	if let Some(term_index) = name.find(0x00 as char){
	    name.truncate(term_index);
	}
	if let Some(term_index) = name.find(0x20 as char){
	    name.truncate(term_index);
	}
	assert!(name.len() > 0);
	
	let mut extension = String::from(String::from_utf8_lossy(&self.file_extension)); // get extension
	// truncate any null terminators
	if let Some(term_index) = extension.find(0x00 as char){
	    extension.truncate(term_index);
	}
	if let Some(term_index) = extension.find(0x20 as char){
	    extension.truncate(term_index);
	}
	
	if extension.len() > 0 {
	    name.push('.');
	    name.push_str(&extension);
	}

	return name;
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

#[repr(C, packed)]
#[derive(Default, Copy, Clone)]
pub struct VFatBlankEntry {
     _res1: [u8; 32],
}

const_assert_size!(VFatUnknownDirEntry, 32);

#[derive(Copy, Clone)]
pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
    blank: VFatBlankEntry
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {

    pub fn from(entry: Entry<HANDLE>) -> Option<Dir<HANDLE>> {
	match entry {
	    Entry::_Dir(dir) => Some(dir),
	    _ => None,
	}
    }
    
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

    // Builds a directory given a root cluster
    // It is the callers responsibility to make sure CLUSTER is a valid root cluster for a directory
    pub fn root(vfat: &HANDLE) -> Entry<HANDLE> {
	Entry::_Dir(Dir {
	    vfat: vfat.clone(),
	    cluster: vfat.lock(|v| v.root_cluster()),
	    metadata: Metadata::root(),
	    short_name: String::new(),
	    long_name: String::new(),
	})
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
	println!("\n\nfile number: {}\n\n", self.entry_offset);

	// deleted entry
	if (entry.file_name[0] == 0xE5 || entry.file_name[0] == 0x00) {
	    return None;
	}
	
	let name = entry.name();
	println!("\n\nname: {:?}\n\n", name);
	
	if entry.metadata.attributes.directory() {
	    let dir_entry = Entry::_Dir(Dir {
	        vfat: self.vfat.clone(),
		cluster: Cluster::from(entry.metadata.cluster()),
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
	while (self.entry_offset < self.entries.len()) {

	    // determine type of entry
	    let mut unknown_entry: &VFatUnknownDirEntry = unsafe {
		&self.entries[self.entry_offset].unknown

	    };
	    
	    // attempt to parse entry
	    if let Some(entry) = {
		// parse LFN
		if unknown_entry.attributes.lfn() {
		    println!("\nparsing lfn with: {}", self.entry_offset);
		    self.parse_lfn()
		} else {
		    println!("\nparsing regular with: {}", self.entry_offset);
		    self.parse_reg(String::from(""))
		}
	    } {
		// return parsed entry or continue to next entry...
		println!("\returning with: {}", self.entry_offset);
		return Some(entry);
	    }	 
	}
	println!("\n failed with: {}", self.entry_offset);
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
	let mut data: Vec<u8> = Vec::new();
	let size = self.vfat.lock(|v| v.read_chain(self.cluster, &mut data))?;
	
	// unsafe cast to Vec::<VFatDirEntry>
	let num_entries: usize = data.len() / size_of::<VFatDirEntry>();
	let mut entries = vec![VFatDirEntry{blank: VFatBlankEntry::default()}; num_entries];
		
	unsafe {
	    data.as_ptr().copy_to(
		entries.as_mut_ptr() as *mut u8,
		num_entries * size_of::<VFatDirEntry>());
	}

	Ok(DirIterator::<HANDLE>{ vfat: self.vfat.clone(), entries: entries, entry_offset: 0})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shim::path;
    use shim::path::Path;
    use shim::io::Cursor;

    use std::sync::{Arc, Mutex};
    use std::fmt::{self, Debug};

    use crate::traits::{BlockDevice, FileSystem};
    use crate::vfat::VFat;
    use crate::traits::Metadata;

    static mut data: [u8; 1024*14] = [0; 1024*14];

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
	    // entry 0 - ID
	    data[fat_start] = 0xFF;
	    data[fat_start + 1] = 0xFF;
	    data[fat_start + 2] = 0xFF;
	    data[fat_start + 3] = 0xFF;

	    // entry - EOC Marker
	    data[fat_start + 4] = 0xF8;
	    data[fat_start + 5] = 0xFF;
	    data[fat_start + 6] = 0xFF;
	    data[fat_start + 7] = 0xFF;

	    // entry 2 - Root
	    data[fat_start + 8] = 0xF8;
	    data[fat_start + 9] = 0xFF;
	    data[fat_start + 10] = 0xFF;
	    data[fat_start + 11] = 0xFF;

	    // entry 3 - file 1 - cluster 2 - EOF
	    data[fat_start + 12] = 0xF8;
	    data[fat_start + 13] = 0xFF;
	    data[fat_start + 14] = 0xFF;
	    data[fat_start + 15] = 0xFF;

	    // entry 4 - file 1 - cluster 1
	    data[fat_start + 16] = 0x03;
	    data[fat_start + 17] = 0;
	    data[fat_start + 18] = 0;
	    data[fat_start + 19] = 0;

	    // entry 5 - file 2 - EOF
	    data[fat_start + 20..fat_start + 24].copy_from_slice(&[0xF8, 0xFF, 0xFF, 0xFF]);

	    // entry 6 - file 2 - EOF
	    data[fat_start + 24..fat_start + 28].copy_from_slice(&[0xF8, 0xFF, 0xFF, 0xFF]);

	    // DATA - Root Dir
	    let cluster_two = ebpb_start + 2*1024;
	    
	    // entry for file 1 - 32 bytes
	    data[cluster_two..cluster_two + 32].copy_from_slice(
		&[0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x00, 0x00, 0x00, // file name
		  0x74, 0x78, 0x74, // file extenstion
		  0x01, // attributes
		  0x00, // reserved
		  99, // creation time in tenths of seconds
		  0x62, 0x04,// time created. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// date on which the file was created. Year: 100 (2080) (0 = 1980). Month: 12. Day: 10.
		  0x62, 0x04,// last accessed time. hours: 14. minutes: 37. seconds: 40
		  0x00, 0x00,// high 16 bits of cluster number
		  0x62, 0x04,// last modified time. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// last modified date. Year: 100 (2080) (0 = 1980). Month: 12. Day: 11.
		  0x04, 0x00,// low 16 bits of cluster number
		  0x00, 0x10, 0x00, 0x00,// size of file in bytes
		]
	    );
	    
	    // entry for file 2
	    data[cluster_two + 32..cluster_two + 32 + 32].copy_from_slice(
		&[0x4E, 0x4F, 0x00, 0x00, 0xFF, 0x32, 0xEC, 0x9A, // file name
		  0x74, 0x78, 0x74, // file extenstion
		  0x01, // attributes
		  0x00, // reserved
		  99, // creation time in tenths of seconds
		  0x62, 0x04,// time created. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// date on which the file was created. Year: 100 (2080) (0 = 1980). Month: 12. Day: 10.
		  0x62, 0x04,// last accessed time. hours: 14. minutes: 37. seconds: 40
		  0, 0,// high 16 bits of cluster number
		  0x62, 0x04,// last modified time. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// last modified date. Year: 100 (2080) (0 = 1980). Month: 12. Day: 11.
		  0x05, 0x00,// low 16 bits of cluster number
		  0x00, 0x08, 0x00, 0x00,// size of file in bytes
		]
	    );
	    
	    // LFN entry for file 3
	    data[cluster_two + 32 + 32..cluster_two + 32 + 32 + 32].copy_from_slice(
		&[0x01,// sequence number
		  0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A,//name_chars_first
		  0x0F,// attributes
		  0x00,// type
		  0, //DOS checksum
		  0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56,//name_chars_second
		  0x00, 0x00,//always 0 for LFN
		  0x57, 0x58, 0x59, 0x5A,//name_chars_third
		]
	    );

	    data[cluster_two + 32 + 32 + 32..cluster_two + 32 + 32 + 32 + 32].copy_from_slice(
		&[0x02,// sequence number
		  0x7A, 0x79, 0x78, 0x77, 0x76, 0x75, 0x74, 0x73, 0x72, 0x71,//name_chars_first
		  0x0F,// attributes
		  0x00,// type
		  0, //DOS checksum
		  0x70, 0x6F, 0x6E, 0x6D, 0x6C, 0x6B, 0x6A, 0x69, 0x68, 0x67, 0x66, 0x65,//name_chars_second
		  0x00, 0x00,//always 0 for LFN
		  0x64, 0x63, 0x62, 0x61,//name_chars_third
		]
	    );

	    data[cluster_two + 32 + 32 + 32 + 32..cluster_two + 32 + 32 + 32 + 32 + 32].copy_from_slice(
		&[0x65, 0x72, 0x69, 0x6E, 0x00, 0x00, 0x00, 0x00, // file name short
		  0x74, 0x78, 0x74, // file extenstion
		  0x01, // attributes
		  0x00, // reserved
		  99, // creation time in tenths of seconds
		  0x62, 0x04,// time created. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// date on which the file was created. Year: 100 (2080) (0 = 1980). Month: 12. Day: 10.
		  0x62, 0x04,// last accessed time. hours: 14. minutes: 37. seconds: 40
		  0, 0,// high 16 bits of cluster number
		  0x62, 0x04,// last modified time. hours: 14. minutes: 37. seconds: 40
		  0x8A, 0xC9,// last modified date. Year: 100 (2080) (0 = 1980). Month: 12. Day: 11.
		  0x06, 0x00,// low 16 bits of cluster number
		  0x00, 0x08, 0x00, 0x00,// size of file in bytes
		]
	    );
	    
	    // File 1 - second cluster
	    let cluster_three = cluster_two + 2*1024;
	    data[cluster_three..cluster_three+4].copy_from_slice(&[99,3,3,3]);
	    data[cluster_three+1024..cluster_three+1028].copy_from_slice(&[33,3,3,3]);

	    // file 1 - first cluster
	    let cluster_four = cluster_three + 2*1024;
	    data[cluster_four..cluster_four+4].copy_from_slice(&[99,4,4,4]);
	    data[cluster_four+1024..cluster_four+1028].copy_from_slice(&[33,4,4,4]);

	    // file 2 - only cluster
	    let cluster_five = cluster_four + 2*1024;
	    data[cluster_five..cluster_five+4].copy_from_slice(&[99,5,5,5]);
	    data[cluster_five+1024..cluster_five+1028].copy_from_slice(&[33,5,5,5]);

	    // file 3 - only cluster
	    let cluster_six = cluster_five + 2*1024;
	    data[cluster_six..cluster_six+4].copy_from_slice(&[99,6,6,6]);
	    data[cluster_six+1024..cluster_six+1028].copy_from_slice(&[33,6,6,6]);
	    
	    

	    Cursor::new(&mut data[..])
	};
	return block_device;
    }

    #[test]
    fn test_dir_mock_parsing() -> Result<(), String> {
	use traits::Entry;
	let block_device = get_block();

	println!("\n\nrunning test_dir_parse\n\n");

	let vfat = VFat::<StdVFatHandle>::from(block_device).expect("failed to initialize VFAT from image");

	let _root = Dir::root(&vfat);
	let root_dir = _root.as_dir().unwrap();
	
	let mut file = root_dir.find("hello.txt").unwrap();
	assert_eq!(file.name(), String::from("hello.txt"));
	assert_eq!(file.metadata().cluster(), 4);
	assert_eq!(file.metadata().file_size(), 4096);
	assert!(file.is_file());

	file = root_dir.find("NO.txt").unwrap();
	assert_eq!(file.name(), String::from("NO.txt"));
	assert_eq!(file.metadata().cluster(), 5);
	assert_eq!(file.metadata().file_size(), 2048);
	assert!(file.is_file());

	file = root_dir.find("erin.txt").unwrap();
	assert_eq!(file.name(), String::from("erin.txt"));
	assert_eq!(file.metadata().cluster(), 6);
	assert_eq!(file.metadata().file_size(), 2048);
	assert!(file.is_file());

	Ok(())
    }
}
