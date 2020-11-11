use alloc::boxed::Box;
use shim::io;
use shim::io::{Read, Write};
use shim::path::Path;
use core::mem;
use core::ptr::Unique;
use core::ops::Add;

use aarch64;
use aarch64::vmsa::*;

use crate::param::*;
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use kernel_api::{OsError, OsResult};

use fat32::traits::FileSystem;
use fat32::traits::{Dir, File, Entry};

use crate ::FILESYSTEM;

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
#[derive(Debug)]
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new() -> OsResult<Process> {
	let trap_frame = TrapFrame::default();

	Ok(Process {
	    context: Box::<TrapFrame>::new(trap_frame),
	    vmap: Box::new(UserPageTable::new()),
	    state: State::Ready,
	})
    }
    
    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut process = Self::do_load(pn)?;

	process.context.sp = Self::get_stack_top().as_u64();
	process.context.elr = Self::get_image_base().as_u64();
	process.context.ttbr0 = VMM.get_baddr().as_u64();
	process.context.ttbr1 = process.vmap.get_baddr().as_u64();	
	process.context.spsr |= aarch64::SPSR_EL1::F | aarch64::SPSR_EL1::A | aarch64::SPSR_EL1::D;

        Ok(process)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use io::Read;
        use alloc::vec;
        use core::cmp::min;
        use crate::FILESYSTEM;
        use fat32::traits::{Entry, FileSystem};

        let mut process = Process::new()?;
        process.vmap.alloc(Self::get_stack_base(), PagePerm::RW);
        
        let entry = FILESYSTEM.open(pn)?;
        let mut file = match entry.into_file() {
            Some(file) => file,
            None => return Err(OsError::NoEntry),
        };
	let file_size = file.size() as usize;
        let mut pos = 0;
        let mut buffer = vec![0u8; file_size];
        loop {
            match file.read(&mut buffer[pos..])? {
                0 => break,
                n => pos += n
            }
        }

        let num_pages = 1 + (file_size / PAGE_SIZE);
        for i in 0..num_pages {
            let base = VirtualAddr::from(USER_IMG_BASE + i * PAGE_SIZE);
            let page = process.vmap.alloc(base, PagePerm::RWX);
            let num_bytes = min(PAGE_SIZE, file_size - i * PAGE_SIZE);
            page[0..num_bytes].copy_from_slice(&buffer[0..num_bytes]);
        }
        
        Ok(process)
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
	VirtualAddr::from(core::usize::MAX)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
	VirtualAddr::from(USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
	VirtualAddr::from(USER_STACK_BASE)
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack. 16-byte aligned.
    pub fn get_stack_top() -> VirtualAddr {
	VirtualAddr::from(core::usize::MAX & (!0xFu64 as usize))
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
	let state = mem::replace(&mut self.state, State::Ready);
	match state {
	    State::Ready => {
		true
	    },
	    
	    State::Waiting(mut event) => {
		if event(self) {
		    true
		} else {
		    mem::replace(&mut self.state, State::Waiting(event));
		    false
		}
	    },
	    
	    State::Running => {
		mem::replace(&mut self.state, State::Running);
		false
	    },
	    
	    State::Dead => {
		mem::replace(&mut self.state, State::Dead);
		false
	    },
	}
    }
    
    pub fn set_exception_link(&mut self, addr: u64) {
	(&mut self.context).elr = addr;
    }
}
