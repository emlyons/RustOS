#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]






#[cfg(not(test))]
mod init;

extern crate alloc;


pub mod allocator;
pub mod console;
// DEBUG pub mod fs;
pub mod mutex;
pub mod shell;

use console::{kprint, kprintln, CONSOLE};
use core::time::Duration;
use pi::timer::spin_sleep;


use allocator::Allocator;
// DEBUG use fs::FileSystem;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
// DEBUG pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {
    /* DEBUG unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }*/

    kprintln!("Welcome to cs3210!");
    shell::shell("> ");
}
