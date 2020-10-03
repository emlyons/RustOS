use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;
use crate::vfat;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    _File(File<HANDLE>),
    _Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.

/// Trait implemented by directory entries in a file system.
///
/// An entry is either a `File` or a `Directory` and is associated with both
/// `Metadata` and a name.
impl <HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
	match self {
	    &Entry::_File(ref file) => &file.name,
	    &Entry::_Dir(ref dir) => &dir.name,
	}
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
	match self {
	    &Entry::_File(ref file) => &file.metadata,
	    &Entry::_Dir(ref dir) => &dir.metadata,
	}
    }
    
    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&Self::File> {
	match self {
	    &Entry::_File(ref file) => Some(file),
	    _ => None,
	}
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&Self::Dir> {
	match self {
	    &Entry::_Dir(ref dir) => Some(dir),
	    _ => None,
	}
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<Self::File> {
	match self {
	    Entry::_File(file) => Some(file),
	    _ => None,
	}
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<Self::Dir> {
	match self {
	    Entry::_Dir(dir) => Some(dir),
	    _ => None,
	}
    }

    /// Returns `true` if this entry is a file or `false` otherwise.
    fn is_file(&self) -> bool {
        self.as_file().is_some()
    }

    /// Returns `true` if this entry is a directory or `false` otherwise.
    fn is_dir(&self) -> bool {
        self.as_dir().is_some()
    }
}

// DEBUG impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    // FIXME: Implement `traits::Entry` for `Entry`.
//}
