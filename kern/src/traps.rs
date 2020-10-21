mod frame;
mod syndrome;
mod syscall;

pub mod irq;
pub use self::frame::TrapFrame;

use pi::interrupt::{Controller, Interrupt};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::shell;


use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

fn handle_synchronous(info: Info, esr: u32, tf: &mut TrapFrame) {    
    match Syndrome::from(esr) {
	Syndrome::Brk(comment) => {
	    shell::shell("$ ");
	},
	_ => {},
    };
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {

    let elr = unsafe {aarch64::ELR_EL1.get() as u64};
    assert_eq!(tf.elr, elr);
    
    match info.kind {
	Kind::Synchronous => {
	    tf.elr += 4;
	    handle_synchronous(info, esr, tf);
	},
	Kind::Irq => {},
	Kind::Fiq => {},
	Kind::SError => {}, 
    };
}