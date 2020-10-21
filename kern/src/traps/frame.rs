use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr: u64,
    spsr: u64,
    sp: u64,
    tpdir: u64,
    q: [u128; 32],
    x: [u64; 30],
    pub lr: u64,
    reserved: u64,
}

const_assert_size!(TrapFrame, 800);

