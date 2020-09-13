#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
pub mod mutex;
pub mod shell;

use core::time::Duration;
use console::kprintln;
use pi::timer::spin_sleep;

const GPIO_BASE: usize = 0x3F000000 + 0x200000;
const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

#[inline(never)]
fn spin_sleep_ms(ms: usize) {
    for _ in 0..(ms * 6000) {
        unsafe { asm!("nop" :::: "volatile"); }
    }
}

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.

unsafe fn kmain() -> ! {

    let sleep_time = Duration::new(1000000, 0);

    const PIN: u32 = 16;
    const SHIFT: u32 = (PIN % 10) * 3;

    // GPIO_16 is OUTPUT
    GPIO_FSEL1.write_volatile (GPIO_FSEL1.read_volatile() & !(0b111 << SHIFT));
    GPIO_FSEL1.write_volatile (GPIO_FSEL1.read_volatile() | (0b001 << SHIFT));
    
       
    // FIXME: STEP 1: Set GPIO Pin 16 as output.
    // FIXME: STEP 2: Continuously set and clear GPIO 16.
    loop {
    
    	 // GPIO_16 is HIGH
	 GPIO_SET0.write_volatile (GPIO_SET0.read_volatile() & !(0b1 << PIN));
	 GPIO_SET0.write_volatile (GPIO_SET0.read_volatile() | (0b1 << PIN));

	 spin_sleep (sleep_time);

	 // GPIO_16 is LOW
	 GPIO_CLR0.write_volatile (GPIO_CLR0.read_volatile() & !(0b1 << PIN));
	 GPIO_CLR0.write_volatile (GPIO_CLR0.read_volatile() | (0b1 << PIN));

	 spin_sleep (sleep_time);
    }
}
