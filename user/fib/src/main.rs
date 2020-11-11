#![feature(asm)]
#![no_std]
#![no_main]

mod cr0;

use kernel_api::println;
use kernel_api::syscall::{getpid, time, exit, write};

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

fn main() {
    //println!("Started...");
    write(72);
    write(101);
    write(108);
    write(108);
    write(111);
    
    let rtn = fib(1);
    if rtn != 1 {
	exit();
    }

    write(72);
    write(101);
    write(108);
    write(108);
    write(111);
    
    //println!("Ended: Result = {}", rtn);
    loop{};
}
