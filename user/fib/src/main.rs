#![feature(asm)]
#![no_std]
#![no_main]

mod cr0;

use kernel_api::print;
use kernel_api::syscall::{getpid, time, exit, write, write_string, vprint};

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

fn main() {
    
    write_string("Started...\n");
    
    let rtn = fib(1);
    if rtn != 1 {
	exit();
    }

    //vprint(format_args!("test: {}", 123));
    //println!("Ended: Result = {}", rtn);
    write_string("Ended: Result = \n");
    loop{};
}
