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
pub mod fs;
pub mod mutex;
pub mod shell;

use console::{kprint, kprintln, CONSOLE};
use core::time::Duration;
use pi::timer::spin_sleep;
use pi::atags;

use allocator::Allocator;
use fs::FileSystem;
use fs::sd::Sd;

//use fat32::traits::BlockDevice;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {

    spin_sleep(Duration::from_secs(1));
    
    // ATAG report
    let atag = atags::Atags::get();
    atag.for_each(|x| kprintln!("{:#?}\n\n", x));

    
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }
    
    kprintln!("
   .~~.   .~~.
  '. \\ ' ' / .'
   .~ .~~~..~.
  : .~.'~'.~. :
 ~ (   ) (   ) ~
( : '~'.~.'~' : )
 ~ .~ (   ) ~. ~
  (  : '~' :  )
   '~ .~~~. ~'
       '~'
Welcome to rustOS on Raspberry Pi!
");

   // let mut sd_test: Sd = Sd::new().unwrap();
   // let mut mbr = [0u8; 512];
   // let bytes_read = sd_test.read_sector(0, &mut mbr).unwrap();
   // assert_eq!(bytes_read, 512);
    
   // kprintln!("\n\n read {} bytes from SD card.\n\n", bytes_read);
    //kprintln!("{:?}\n", &mbr[0..512]);
    
    shell::shell("> ");
}
