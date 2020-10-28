use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::{Process, State};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use pi::timer::current_time;
use kernel_api::*;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {

    if ms == 0 {
	SCHEDULER.switch(State::Ready, tf);
	return;
    }
    
    let start_time = current_time();
    let wakeup_time = start_time + Duration::from_millis(ms as u64);

    let wakeupFn = Box::new(move |process: &mut Process| {
	let current_time = current_time();
	if current_time >= wakeup_time {
	    process.context.x[0] = (current_time - start_time).as_millis() as u64;
	    process.context.x[7] = OsError::Ok as u64;
	    return true;
	} else {
	    return false;
	}
    });
    SCHEDULER.switch(State::Waiting(wakeupFn), tf);
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    let time = current_time();
    let seconds = time.as_secs();
    let nano_fraction = time.subsec_nanos();

    tf.x[0] = seconds;
    tf.x[1] = nano_fraction as u64;
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    unimplemented!("sys_exit()");
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    unimplemented!("sys_write()");
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    unimplemented!("sys_getpid()");
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    match (num as usize) {
	NR_SLEEP => {
	    let time = tf.x[0];
	    sys_sleep(time as u32, tf);
	},
	_ => {
	    // error code
	},
    }
}
