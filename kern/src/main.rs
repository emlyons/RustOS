#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#[cfg(not(test))]
mod init;

extern crate alloc;
#[macro_use]
extern crate log;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod logger;
pub mod mutex;
pub mod net;
pub mod param;
pub mod percore;
pub mod process;
pub mod shell;
pub mod traps;
pub mod vm;

use console::{kprint, kprintln, CONSOLE};
use core::time::Duration;
use pi::timer::spin_sleep;
use pi::atags;
use allocator::Allocator;
use fs::FileSystem;
use net::uspi::Usb;
use net::GlobalEthernetDriver;
use process::GlobalScheduler;
use traps::irq::{Fiq, GlobalIrq};
use vm::VMManager;
use aarch64::*;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static USB: Usb = Usb::uninitialized();
pub static GLOBAL_IRQ: GlobalIrq = GlobalIrq::new();
pub static FIQ: Fiq = Fiq::new();
pub static ETHERNET: GlobalEthernetDriver = GlobalEthernetDriver::uninitialized();

extern "C" {
    static __text_beg: u64;
    static __text_end: u64;
    static __bss_beg: u64;
    static __bss_end: u64;
}

unsafe fn kmain() -> ! {
    crate::logger::init_logger();

    info!(
        "text beg: {:016x}, end: {:016x}",
        &__text_beg as *const _ as u64, &__text_end as *const _ as u64
    );
    info!(
        "bss  beg: {:016x}, end: {:016x}",
        &__bss_beg as *const _ as u64, &__bss_end as *const _ as u64
    );
    
    spin_sleep(Duration::from_secs(1));
    
    // ATAG report
    //let atag = atags::Atags::get();
    //atag.for_each(|x| kprintln!("{:#?}\n\n", x));

    unsafe {
	kprint!("initializing memory allocator... ");
	ALLOCATOR.initialize();
	kprintln!("ready");

	kprint!("initializing file system... ");
        FILESYSTEM.initialize();
	kprintln!("ready");

	//kprint!("initializing irq handler... ");
	//GLOBAL_IRQ.initialize();
	//kprintln!("ready");

	kprint!("initializing virtual memory manager... ");
	VMM.initialize();
	VMM.setup();
	kprintln!("ready");

	kprint!("initializing scheduler... ");
	SCHEDULER.initialize();
	kprintln!("ready\n\n");

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

	SCHEDULER.start();
    }

    loop {
	shell::shell(">");
    }
}

// TODO: TEMP
pub extern "C" fn temp_shell() {
    use crate::shell;
    loop {
	shell::shell("$");
    }
}
