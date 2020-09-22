use core::{slice, str};
use crate::atags::raw;

pub use crate::atags::raw::{Core, Mem};

/// An ATAG.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Atag {
    Core(raw::Core),
    Mem(raw::Mem),
    Cmd(&'static str),
    Unknown(u32),
    None,
}

impl Atag {
    /// Returns `Some` if this is a `Core` ATAG. Otherwise returns `None`.
    pub fn core(self) -> Option<Core> {
        match self {
	    Atag::Core(_core) => Some(_core),
	    _ => None,
	}
    }

    /// Returns `Some` if this is a `Mem` ATAG. Otherwise returns `None`.
    pub fn mem(self) -> Option<Mem> {
        match self {
	    Atag::Mem(_mem) => Some(_mem),
	    _ => None,
	}
    }

    /// Returns `Some` with the command line string if this is a `Cmd` ATAG.
    /// Otherwise returns `None`.
    pub fn cmd(self) -> Option<&'static str> {
        match self {
	    Atag::Cmd(_cmd) => Some(_cmd),
	    _ => None,
	}
    }
}

// FIXME: Implement `From<&raw::Atag> for `Atag`.
impl From<&'static raw::Atag> for Atag {
    fn from(atag: &'static raw::Atag) -> Atag {
        // FIXME: Complete the implementation below.

        unsafe {
            match (atag.tag, &atag.kind) {
                (raw::Atag::CORE, &raw::Kind { core }) => Atag::Core(core),
                (raw::Atag::MEM, &raw::Kind { mem }) => Atag::Mem(mem),
                (raw::Atag::CMDLINE, &raw::Kind { ref cmd }) => {

		    // cast cmd byte into [u8] of atag.dwords size
		    let cmd_len: usize = (atag.dwords as usize - 2) * 4;
		    let cmd_ptr = cmd as *const raw::Cmd as *const u8;

		    // verify null terminator
		    let index = slice::from_raw_parts(cmd_ptr, cmd_len).iter().position(|&x| x == '\0' as u8).unwrap();

		    // cast [u8] into str
		    let cmd_slice = slice::from_raw_parts(cmd_ptr, index + 1);
		    let cmd_str = str::from_utf8(cmd_slice).unwrap(); 
		    Atag::Cmd(cmd_str)
		},
                (raw::Atag::NONE, _) => Atag::None,
                (id, _) => Atag::Unknown(id),
            }
        }
    }
}

// cmd as *const u8
