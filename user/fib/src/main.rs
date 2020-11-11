#![feature(asm)]
#![no_std]
#![no_main]

use core::fmt::Arguments;
use core::fmt;
use core::fmt::Write;
    

mod cr0;

use kernel_api::print;
use kernel_api::syscall::{getpid, sleep, time, exit, write, write_string, vprint, stack_info};
use core::time::Duration;

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}
/*
fn fact(n: u64) -> u64 {
    let mut val = 1;
    let mut val2 = 1;
    if n > 1 {
	val = n * fact(n - 1);
    } 
    return val;
}

fn int_to_str<'a>(n: &'a u64) -> &'a str {
    match n {
	0 => "0",
	1 => "1",
	2 => "2",
	3 => "3",
	4 => "4",
	5 => "5",
	6 => "6",
	7 => "7",
	8 => "8",
	9 => "9",
	10 => "10",
	11 => "11",
	12 => "12",
	13 => "13",
	14 => "14",
	15 => "15",
	16 => "16",
	17 => "17",
	18 => "18",
	19 => "19",
	20 => "20",
	21 => "21",
	22 => "22",
	23 => "23",
	24 => "24",
	25 => "25",
	26 => "26",
	27 => "27",
	28 => "28",
	29 => "29",
	30 => "30",
	_ => "other",
    }
}
*/

fn main() {
    loop {
	//let x = fact(2);
	//let x_str = int_to_str(&x);

	stack_info();

	write_string("\nresult = ");
	//write_string(x_str);
	write_string("\nmore\n");

	
	//assert!(fact(10) > 0);
	sleep(Duration::from_secs(2));
    };
}
