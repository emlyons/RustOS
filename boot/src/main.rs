#![feature(asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

//use volatile::prelude::*;
use volatile::{Volatile, WriteVolatile, ReadVolatile, Reserved};
use xmodem::Xmodem;
use core::time::Duration;
use core::slice;
use pi::uart::MiniUart;
use pi::gpio::Gpio;
use pi::timer::spin_sleep;

/// Start address of the binary to load and of the bootloader.
const BINARY_START_ADDR: usize = 0x80000;
const BOOTLOADER_START_ADDR: usize = 0x4000000;

/// Pointer to where the loaded binary expects to be laoded.
const BINARY_START: *mut u8 = BINARY_START_ADDR as *mut u8;

/// Free space between the bootloader and the loaded binary's start address.
const MAX_BINARY_SIZE: usize = BOOTLOADER_START_ADDR - BINARY_START_ADDR;

/// Branches to the address `addr` unconditionally.
unsafe fn jump_to(addr: *mut u8) -> ! {
    asm!("br $0" : : "r"(addr as usize));
    loop {
        asm!("wfe" :::: "volatile")
    }
}

fn kmain() -> ! {
    
    let mut notify_led = Gpio::new(5).into_output();
    let mut xmodem_led = Gpio::new(6).into_output();
    let mut uart = MiniUart::new();
    uart.set_read_timeout(Duration::from_millis(750));
 
    loop {
	// FIXME: Implement the bootloader.

	xmodem_led.set();
	
	// want pointer to 0x80000
	// limit transfer size to = 0x4000000 - 0x80000 = 66584576
	// use [WriteVolatile<u8>; 66584576]
	
	//	let boot_loc: [WriteVolatile<u8>; MAX_BINARY_SIZE] = unsafe{ &mut *(BINARY_START_ADDR as [u8].as_ptr()) };
	//let boot_loc: [WriteVolatile<u8>; MAX_BINARY_SIZE] =
	//let boot_loc = unsafe{ slice::from_raw_parts_mut(BINARY_START_ADDR, MAX_BINARY_SIZE) };
//	let boot_addr = BINARY_START_ADDR as *mut u8;
	let mut boot_loc = unsafe{slice::from_raw_parts_mut(BINARY_START, MAX_BINARY_SIZE)};
	
	match Xmodem::receive(&mut uart, &mut boot_loc) {
	    Ok(_ok) => {
		notify_led.set();
		xmodem_led.clear();
		unsafe{jump_to (BINARY_START)};
	    },
	    Err(_err) => {
		continue;
	    },
	}

	spin_sleep(Duration::from_millis(400));
	xmodem_led.clear();
	spin_sleep(Duration::from_millis(400));
       
    }
}
