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
use console::{kprint, kprintln};
use pi::timer::spin_sleep;
use pi::gpio;
use pi::uart;


fn binary_led(val: u8) {
    
}

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.

unsafe fn kmain() -> ! {

    let sleep_time = Duration::from_millis(50);
 
    fn binary_led(val: u8) {

	let mut gpio_5 = gpio::Gpio::new(5).into_output();
	let mut gpio_6 = gpio::Gpio::new(6).into_output();
	let mut gpio_13 = gpio::Gpio::new(13).into_output();
	let mut gpio_16 = gpio::Gpio::new(16).into_output();
	let mut gpio_19 = gpio::Gpio::new(19).into_output();
	let mut gpio_26 = gpio::Gpio::new(26).into_output();
	
	if (val & 0b1) == 0b1 {
	    gpio_5.set();   
	}
	else {
	    gpio_5.clear();
	}

	if (val & 0b10) == 0b10 {
	    gpio_6.set();   
	}
	else {
	    gpio_6.clear();
	}

	if (val & 0b100) == 0b100 {
	    gpio_13.set();   
	}
	else {
	    gpio_13.clear();
	}

	if (val & 0b1000) == 0b1000 {
	    gpio_16.set();   
	}
	else {
	    gpio_16.clear();
	}

	if (val & 0b10000) == 0b10000 {
	    gpio_19.set();   
	}
	else {
	    gpio_19.clear();
	}

	if (val & 0b100000) == 0b100000 {
	    gpio_26.set();   
	}
	else {
	    gpio_26.clear();
	}	    
    }

       
    // FIXME: STEP 1: Set GPIO Pin 16 as output.
    // FIXME: STEP 2: Continuously set and clear GPIO 16.
    let mut led_val = 0;

    let mut uart = uart::MiniUart::new();
    
    loop {

	// echo
	let new_byte = uart.read_byte();
	kprint!("{}", new_byte);

	// binary counter
	binary_led(led_val);
	spin_sleep (sleep_time);
	led_val = (led_val + 1) % 64
	
    }
}
