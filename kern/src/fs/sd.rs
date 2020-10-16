use core::time::Duration;
use shim::io;
use pi::timer::spin_sleep;

use fat32::traits::BlockDevice;

extern "C" {
    /// A global representing the last SD controller error that occured.
    static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory. Also, the caller of this function should make sure that
    /// `buffer` is at least 4-byte aligned.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.
    fn sd_readsector(n: i32, buffer: *mut u8) -> i32;
}

// FIXME: Define a `#[no_mangle]` `wait_micros` function for use by `libsd`.
// The `wait_micros` C signature is: `void wait_micros(unsigned int);`
#[no_mangle]
fn wait_micros(us: u32) {
    let wait_time = Duration::from_micros(us as u64);
    spin_sleep(wait_time);
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization. We can enforce the requirement in safe Rust code
    /// with atomic memory access, but we can't use it yet since we haven't
    /// written the memory management unit (MMU).
    pub fn new() -> Result<Sd, io::Error> {
	let result = unsafe{
	    sd_init()
	};
	if result == 0 {
	    return Ok(Sd{});
	}
	if result == -1 {
	    return Err(io::Error::new(io::ErrorKind::TimedOut, "SD card controller timed out")); 
	}
	if result == -2 {
	    return Err(io::Error::new(io::ErrorKind::Other, "communication failed with SD card controller"));
	}
	Err(io::Error::new(io::ErrorKind::Other, "an undefined error occured"))
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
	if (buf.len() as u64) < self.sector_size() {
	    return Err(io::Error::new(io::ErrorKind::InvalidInput, "buffer too small to read sector"));
	}

	if n > 0x7FFFFFFF {
	    return Err(io::Error::new(io::ErrorKind::InvalidInput, "out of range sector requested for read"));
	}

	let bytes_read = unsafe{
	    sd_readsector(n as i32, buf.as_mut_ptr())
	};

	if bytes_read == 0 {
	    // error occurred
	    unsafe {
		match sd_err {
		    -1 => {return Err(io::Error::new(io::ErrorKind::TimedOut, "SD card controller timed out"));},
		    -2 => {return Err(io::Error::new(io::ErrorKind::Other, "communication failed with SD card controller"));},
		    _ => {return Err(io::Error::new(io::ErrorKind::Other, "an undefined error occured"));},
		};
	    }
	}

	if bytes_read < 0 {
	    // undefined behaviour
	    return Err(io::Error::new(io::ErrorKind::Other, "an undefined error occured"));
	}

	// successfull read
	Ok(bytes_read as usize)
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
