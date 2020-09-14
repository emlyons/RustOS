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
use pi::gpio;

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.

unsafe fn kmain() -> ! {

    let sleep_time = Duration::from_secs(100000);

    // pin 5
    let mut GPIO_5 = gpio::Gpio::new(5).into_output();
    // pin 6
    let mut GPIO_6 = gpio::Gpio::new(6).into_output();
    // pin 13
    let mut GPIO_13 = gpio::Gpio::new(13).into_output();
    // pin 16
    let mut GPIO_16 = gpio::Gpio::new(16).into_output();
    // pin 19
    let mut GPIO_19 = gpio::Gpio::new(19).into_output();
    // pin 26
    let mut GPIO_26 = gpio::Gpio::new(26).into_output();

       
    // FIXME: STEP 1: Set GPIO Pin 16 as output.
    // FIXME: STEP 2: Continuously set and clear GPIO 16.
    loop {

	GPIO_5.set();

	spin_sleep (sleep_time);

	GPIO_6.set();
	GPIO_5.clear();

	spin_sleep (sleep_time);

	GPIO_13.set();
	GPIO_6.clear();

	spin_sleep (sleep_time);

	GPIO_16.set();
	GPIO_13.clear();

	spin_sleep (sleep_time);

	GPIO_19.set();
	GPIO_16.clear();

	spin_sleep (sleep_time);

	GPIO_26.set();
	GPIO_19.clear();

	spin_sleep (sleep_time);

	GPIO_26.clear();
	
    }
}
