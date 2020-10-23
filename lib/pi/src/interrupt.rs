use crate::common::IO_BASE;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, Reserved};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[derive(Copy, Clone, PartialEq)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

impl Interrupt {
    pub const MAX: usize = 8;

    pub fn iter() -> core::slice::Iter<'static, Interrupt> {
        use Interrupt::*;
        [Timer1, Timer3, Usb, Gpio0, Gpio1, Gpio2, Gpio3, Uart].into_iter()
    }

    pub fn to_index(i: Interrupt) -> usize {
        use Interrupt::*;
        match i {
            Timer1 => 0,
            Timer3 => 1,
            Usb => 2,
            Gpio0 => 3,
            Gpio1 => 4,
            Gpio2 => 5,
            Gpio3 => 6,
            Uart => 7,
        }
    }

    pub fn from_index(i: usize) -> Interrupt {
        use Interrupt::*;
        match i {
            0 => Timer1,
            1 => Timer3,
            2 => Usb,
            3 => Gpio0,
            4 => Gpio1,
            5 => Gpio2,
            6 => Gpio3,
            7 => Uart,
            _ => panic!("Unknown interrupt: {}", i),
        }
    }
}


impl From<usize> for Interrupt {
    fn from(irq: usize) -> Interrupt {
        use Interrupt::*;
        match irq {
            1 => Timer1,
            3 => Timer3,
            9 => Usb,
            49 => Gpio0,
            50 => Gpio1,
            51 => Gpio2,
            52 => Gpio3,
            57 => Uart,
            _ => panic!("Unknown irq: {}", irq),
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IRQ_PND_BASIC: ReadVolatile<u32>,
    IRQ_PND_1: ReadVolatile<u32>,
    IRQ_PND_2: ReadVolatile<u32>,
    FIQ_CTRL: Volatile<u32>,
    IRQ_ENBL_1: Volatile<u32>,
    IRQ_ENBL_2: Volatile<u32>,
    __r0: [Reserved<u8>; 3],
    IRQ_ENBL_BASIC: Volatile<u8>,
    IRQ_DSBL_1: Volatile<u32>,
    IRQ_DSBL_2: Volatile<u32>,
    __r1: [Reserved<u8>; 3],
    IRQ_DSBL_BASIC: Volatile<u8>,
    
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
	let irq_index = int as u32;
	let mask = irq_index % 32;

	let irq_enable = match irq_index {
	    i if i < 32 => &mut self.registers.IRQ_ENBL_1,
	    i if i < 64 => &mut self.registers.IRQ_ENBL_2,
	    _ => unreachable!(),
	};
	
	irq_enable.or_mask(mask);
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
	let irq_index = int as u32;
	let mask = irq_index % 32;

	let irq_disable = match irq_index {
	    i if i < 32 => &mut self.registers.IRQ_DSBL_1,
	    i if i < 64 => &mut self.registers.IRQ_DSBL_2,
	    _ => unreachable!(),
	};
	
	irq_disable.or_mask(mask);
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
	let irq_index = int as u32;
	let mask = irq_index % 32;

	let irq_pending = match irq_index {
	    i if i < 32 => &self.registers.IRQ_PND_1,
	    i if i < 64 => &self.registers.IRQ_PND_2,
	    _ => unreachable!(),
	};
	
	irq_pending.has_mask(mask)
    }
}
