use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpdir: u64,
    pub q: [u128; 32],
    pub x: [u64; 30],
    pub lr: u64,
    pub reserved: u64,
}

const_assert_size!(TrapFrame, 800);

impl TrapFrame {
    pub fn new_zeroed() -> TrapFrame {
	TrapFrame {
	    elr: 0,
	    spsr: 0,
	    sp: 0,
	    tpdir: 0,
	    q: [0u128; 32],
	    x: [0u64; 30],
	    lr: 0,
	    reserved: 0,
	}
    }

    pub fn set_elr(&mut self, val: u64) {
	self.elr = val;
    }

    pub fn get_elr(&self) -> u64 {
	self.elr
    }

    pub fn set_spsr(&mut self, val: u64) {
	self.spsr = val;
    }

    pub fn get_spsr(&self) -> u64 {
	self.spsr
    }

    pub fn set_sp(&mut self, val: u64) {
	self.sp = val;
    }

    pub fn get_sp(&self) -> u64 {
	self.sp
    }

    pub fn set_tpdir(&mut self, val: u64) {
	self.tpdir = val;
    }

    pub fn get_tpdir(&self) -> u64 {
	self.tpdir
    }

    pub fn set_q(&mut self, num: usize, val: u128) {
	self.q[num] = val;
    }

    pub fn get_q(&self, num: usize) -> u128 {
	self.q[num]
    }

    pub fn set_x(&mut self, num: usize, val: u64) {
	self.x[num] = val;
    }

    pub fn get_x(&self, num: usize) -> u64 {
	self.x[num]
    }

    pub fn set_lr(&mut self, val: u64) {
	self.lr = val;
    }

    pub fn get_lr(&self) -> u64 {
	self.lr
    }
}

