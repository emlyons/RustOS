use core::time::Duration;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, WriteVolatile, Reserved};

use aarch64::*;

const INT_BASE: usize = 0x40000000;

/// Core interrupt sources (QA7: 4.10)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LocalInterrupt {
    CNTPSIRQ,
    CNTPNSIRQ,
    CNTHPIRQ,
    CNTVIRQ,
    MAILBOX_0,
    MAILBOX_1,
    MAILBOX_2,
    MAILBOX_3,
    GPU,
    PMU,
    AXI,
    LOCAL_TIMER,
}

impl LocalInterrupt {
    pub const MAX: usize = 12;

    pub fn iter() -> impl Iterator<Item = LocalInterrupt> {
        (0..LocalInterrupt::MAX).map(|n| LocalInterrupt::from(n))
    }
}

impl From<usize> for LocalInterrupt {
    fn from(irq: usize) -> LocalInterrupt {
	use LocalInterrupt::*;
        match irq {
            0 => CNTPSIRQ,
	    1 => CNTPNSIRQ,
	    2 => CNTHPIRQ,
	    3 => CNTVIRQ,
	    4 => MAILBOX_0,
	    5 => MAILBOX_1,
	    6 => MAILBOX_2,
	    7 => MAILBOX_3,
	    8 => GPU,
	    9 => PMU,
	    10 => AXI,
	    11 => LOCAL_TIMER,
	    _ => panic!("Unknown irq: {}", irq),
        }
    }
}

/// BCM2837 Local Peripheral Registers (QA7: Chapter 4)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // Lab 5 1.C
    CONTROL_REGISTER: Volatile<u32>,
    __r0: Reserved<u32>,
    CORE_TIMER_PRESCALER: Volatile<u32>,
    GPU_INTERRUPTS_ROUTING: Volatile<u32>,
    INTERRUPTS_ROUTING_SET: WriteVolatile<u32>,
    INTERRUPTS_ROUTING_CLEAR: WriteVolatile<u32>,
    __r1: Reserved<u32>,
    CORE_TIMER_ACCESS_LS: ReadVolatile<u32>,
    CORE_TIMER_ACCESS_MS: ReadVolatile<u32>,
    LOCAL_INTERRUPTS_1_7_ROUTING: Volatile<u32>,
    LOCAL_INTERRUPTS_8_15_ROUTING: Volatile<u32>,
    AXI_OUTSTANDING_COUNTERS: ReadVolatile<u32>,
    AXI_OUTSTANDING_IRQ: ReadVolatile<u32>,
    LOCAL_TIMER_CONTROL_AND_STATUS: Volatile<u32>,
    LOCAL_TIMER_WRITE_FLAGS: Volatile<u32>,
    __r2: Reserved<u32>,
    CORE_0_TIMERS_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_1_TIMERS_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_2_TIMERS_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_3_TIMERS_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_0_MAILBOXES_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_1_MAILBOXES_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_2_MAILBOXES_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_3_MAILBOXES_INTERRUPT_CONTROL: Volatile<u32>,
    CORE_0_IRQ_SOURCE: ReadVolatile<u32>,
    CORE_1_IRQ_SOURCE: ReadVolatile<u32>,
    CORE_2_IRQ_SOURCE: ReadVolatile<u32>,
    CORE_3_IRQ_SOURCE: ReadVolatile<u32>,
    CORE_0_FIQ_SOURCE: ReadVolatile<u32>,
    CORE_1_FIQ_SOURCE: ReadVolatile<u32>,
    CORE_2_FIQ_SOURCE: ReadVolatile<u32>,
    CORE_3_FIQ_SOURCE: ReadVolatile<u32>,
}

pub struct LocalController {
    core: usize,
    registers: &'static mut Registers,
}

impl LocalController {
    /// Returns a new handle to the interrupt controller.
    pub fn new(core: usize) -> LocalController {
        LocalController {
            core: core,
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    pub fn enable_local_timer(&mut self) {
        // Lab 5 1.C

	// enable timer
	unsafe {
	    CNTP_CTL_EL0.set(CNTP_CTL_EL0::ENABLE)
	};
	
	// enable timer interrupt
	match self.core {
	    0 => self.registers.CORE_0_TIMERS_INTERRUPT_CONTROL.write(0b10),
	    1 => self.registers.CORE_1_TIMERS_INTERRUPT_CONTROL.write(0b10),
	    2 => self.registers.CORE_2_TIMERS_INTERRUPT_CONTROL.write(0b10),
	    3 => self.registers.CORE_3_TIMERS_INTERRUPT_CONTROL.write(0b10),
	    _ => unreachable!(),
	}
    }

    pub fn is_pending(&self, int: LocalInterrupt) -> bool {
        // Lab 5 1.C

	// Read corresponding bits from Core X interrupt source register (QA7: 4.10) and convert it to a boolean value.
	let pending: u32 = match self.core {
	    0 => self.registers.CORE_0_IRQ_SOURCE.read(),
	    1 => self.registers.CORE_1_IRQ_SOURCE.read(),
	    2 => self.registers.CORE_2_IRQ_SOURCE.read(),
	    3 => self.registers.CORE_3_IRQ_SOURCE.read(),
	    _ => unreachable!(),
	};

	// check for CNTPNSIRQ
	if pending == 1 {
	    true
	} else {
	    false
	}
    }

    pub fn tick_in(&mut self, t: Duration) {
        // Lab 5 1.C
	let clock_freq: u64 = unsafe {
	    CNTFRQ_EL0.get()
	};

	// convert to to number of ticks
	let tick_number = match t.checked_mul(clock_freq as u32) {
	    Some(d) => d.as_secs(),
	    None => 0,
	};

	// set trigger time
	unsafe {
	    CNTP_TVAL_EL0.set(tick_number);
	}
    }
}

pub fn local_tick_in(core: usize, t: Duration) {
    LocalController::new(core).tick_in(t);
}
