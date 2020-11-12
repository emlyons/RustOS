#![feature(asm)]
#![no_std]
#![no_main]

mod cr0;

use kernel_api::println;
use kernel_api::syscall::{getpid, time, sleep, exit};
use core::time::Duration;

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

fn main() {
    loop {
	let time = time();
	println!("Started...{}", time.as_secs());

	let pid = getpid();
	println!("pid: {}", pid);

	let rtn = fib(40);

	println!("Ended: Result = {}", rtn);

	if time.as_secs() > 5 {
	    exit();
	}
    }
}
