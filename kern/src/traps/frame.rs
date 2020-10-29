use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub ttbr0: u64,
    pub ttbr1: u64,
    pub q: [u128; 32],
    pub x: [u64; 30],
    pub lr: u64,
    pub reserved: u64,
}

const_assert_size!(TrapFrame, 816);

impl TrapFrame {
    pub fn new_zeroed() -> TrapFrame {
	TrapFrame {
	    elr: 0,
	    spsr: 0,
	    sp: 0,
	    tpidr: 0,
	    ttbr0: 0,
	    ttbr1: 0,
	    q: [0u128; 32],
	    x: [0u64; 30],
	    lr: 0,
	    reserved: 0,
	}
    }
}
