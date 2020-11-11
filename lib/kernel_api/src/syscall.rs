use core::fmt;
use core::fmt::Write;
use core::time::Duration;

use crate::*;

macro_rules! err_or {
    ($ecode:expr, $rtn:expr) => {{
        let e = OsError::from($ecode);
        if let OsError::Ok = e {
            Ok($rtn)
        } else {
            Err(e)
        }
    }}
}

pub fn sleep(span: Duration) -> OsResult<Duration> {
    if span.as_millis() > core::u64::MAX as u128 {
        panic!("too big!");
    }

    let ms = span.as_millis() as u64;
    let mut ecode: u64;
    let mut elapsed_ms: u64;
    
    unsafe {
        asm!("svc $2"
             : "={x0}"(elapsed_ms), "={x7}"(ecode)
             : "i"(NR_SLEEP), "{x0}"(ms)
             : "x0", "x7", "memory"
             : "volatile");
    }

    err_or!(ecode, Duration::from_millis(elapsed_ms))
}

pub fn time() -> Duration {
    unimplemented!("time()");
}

pub fn exit() -> ! {
    unsafe {
        asm!("svc $0"
             :
	     : "i"(NR_EXIT)
             : "memory"
             : "volatile");
    }
    loop {};
}

pub fn write(b: u8) {
    let mut ecode: u64 = 0;
    
    unsafe {
        asm!("mov x0, $1
              svc $0
              mov x7, #0
              mov x0, #0"
             :
             : "i"(NR_WRITE), "r"(b)
             : "x0", "x7"
	     : "volatile"
	);
    }

}

pub fn getpid() -> u64 {
    unimplemented!("getpid()");
}


struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            write(b);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::syscall::vprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
 () => (print!("\n"));
    ($($arg:tt)*) => ({
        $crate::syscall::vprint(format_args!($($arg)*));
        $crate::print!("\n");
    })
}

pub fn vprint(args: fmt::Arguments) {
    let mut c = Console;
    c.write_fmt(args).unwrap();
}
