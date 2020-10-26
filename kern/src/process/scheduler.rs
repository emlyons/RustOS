use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;
use core::mem::replace;

use aarch64::*;

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::VMM;
use crate::IRQ;

use pi::interrupt::{Interrupt, Controller};
use pi::timer::{tick_in, current_time};

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            aarch64::wfe();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {

	// systick
	IRQ.register(Interrupt::Timer1, Box::new(systick_handler));
	Controller::new().enable(Interrupt::Timer1);
	tick_in(TICK);

	//let mut scheduler = Scheduler::new();
	let mut process = Process::new().expect("failed to allocate memory for new process");	
	process.context.elr = temp_shell as *mut u8 as u64;
	process.context.spsr = 0b1101000000;
	process.context.sp = process.stack.top().as_u64();

	unsafe{
	    asm!("mov sp, $0
		 bl context_restore
		 adr lr, _start
		 mov sp, lr
	         mov lr, xzr
                 eret" :: "r"(&*process.context) :: "volatile");
	};

	loop {};
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        unimplemented!("GlobalScheduler::initialize()")
    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    // pub fn test_phase_3(&self, proc: &mut Process){
    //     use crate::vm::{VirtualAddr, PagePerm};
    //
    //     let mut page = proc.vmap.alloc(
    //         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
    //
    //     let text = unsafe {
    //         core::slice::from_raw_parts(test_user_process as *const u8, 24)
    //     };
    //
    //     page[0..24].copy_from_slice(text);
    // }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
	Scheduler {
	    processes: VecDeque::<Process>::new(),
	    last_id: Some(0),
	}
    }

    fn next_id(&mut self) -> Option<Id> {
	let last_id = self.last_id?;
	let next_id = last_id.checked_add(1)?;
	Some(next_id)
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
	let id = self.next_id()?;
	process.context.tpdir = id;
	self.processes.push_back(process);
	Some(id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {	
	for index in 0..self.processes.len(){
	    match self.processes[index].state {
		State::Running => {
		    if self.processes[index].context.tpdir == tf.tpdir {
			let mut process = self.processes.remove(index).expect("removing sheduled out process from queue");
			process.state = new_state;
			replace(&mut *process.context, *tf);
			self.processes.push_back(process);
			return true;
		    }
		},
		_ => continue,// TODO: can break after verification
	    }
	}
	
	false
    }
    
    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {

	for index in 0..self.processes.len(){
	    if self.processes[index].is_ready() {
		let mut process = self.processes.remove(index).expect("removing sheduled out process from queue");
		process.state = State::Running;
		replace(&mut *tf, *process.context);
		assert_eq!(tf.tpdir, process.context.tpdir);
		self.processes.push_front(process);
		return Some(tf.tpdir);
	    }
	}
	None
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
	if self.schedule_out(State::Dead, tf) {
	    let process = self.processes.pop_back().expect("removing process on kill");
	    assert_eq!(tf.tpdir, process.context.tpdir);
	    Some(tf.tpdir)
	}
	else {
	    None
	}
    }
}

pub extern "C" fn  test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}

// TODO: TEMP
#[no_mangle]
pub extern "C" fn temp_shell() {
    use crate::shell;
    loop {
	shell::shell("$");
    }
}

// TODO: SYSTICK HANDLER should go where?
pub fn systick_handler(tf: &mut TrapFrame) {
    tick_in(TICK);
}
