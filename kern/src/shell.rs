use stack_vec::StackVec;

use crate::console::{kprint, kprintln, CONSOLE};

use shim::io::Write;
use core::str;

use pi::gpio;

const NEWLINE: u8 = 10;
const RETURN: u8 = 13;
const BACKSPACE: u8 = 08;
const DELETE: u8 = 127;
const BELL: u8 = 7;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
	assert!(!self.args.is_empty());
	self.args[0]
    }

    /// fullfills command request if present/valid in Command struct
    fn execute(&self) {
	match self.path() {
	    "echo" => self.echo(),
	    "binled" => self.binary_led(),
	    _ => {
		kprintln!("");
		kprint!("unknown command");
	    },
	}
    }

    fn echo (&self) {
	assert_eq!(self.args[0], "echo");
	kprintln!("");
	self.args.as_slice().iter().skip(1).for_each(|arg| kprint!("{}", arg));
    }

    fn binary_led(&self) {
	assert_eq!(self.args[0], "binled");

	// parse number from string, if unsuccessful turn off leds
	let mut val: u8 = 0;
	let arg = self.args[1].parse::<u8>();
	if arg.is_ok() {
	    val = arg.unwrap();
	}

	let mut _gpio = [gpio::Gpio::new(5).into_output(),
		     gpio::Gpio::new(6).into_output(),
		     gpio::Gpio::new(13).into_output(),
		     gpio::Gpio::new(16).into_output(),
		     gpio::Gpio::new(19).into_output(),
		     gpio::Gpio::new(26).into_output()];

	_gpio.iter_mut().enumerate().for_each(|(i, pin)| {
	    if (val & (0b1 << i)) == (0b1 << i) {
		pin.set()
	    } else {
		pin.clear()}
	})	    
    }

}


/*

    [] implement the echo built-in: echo $a $b $c should print $a $b $c

    [X] accept both \r and \n as “enter”, marking the end of a line

    [X] accept both backspace and delete (ASCII 8 and 127) to erase a single character

    [X] ring the bell (ASCII 7) if an unrecognized non-visible character is sent to it

    [X] print unknown command: $command for an unknown command $command

    [X] disallow backspacing through the prefix

    [X] disallow typing more characters than allowed

    [X] accept commands at most 512 bytes in length

    [X] accept at most 64 arguments per command

    [X] start a new line, without error, with the prefix if the user enters an empty command

    [X] print error: too many arguments if the user passes in too many arguments

 */


/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell(prefix: &str) -> ! {

    let mut buff_backing = [0u8; 512];
    let mut buf = StackVec::new(&mut buff_backing);

    kprint!("{}", prefix);
    
    loop {
	// print out prefix
	let mut console = CONSOLE.lock();
	let new_byte = console.read_byte();

	match new_byte {
	    
	    byte if (byte == NEWLINE || byte == RETURN) => {
		let mut cmd_backing: [&str; 64] = [""; 64];
		let command = Command::parse(str::from_utf8(buf.as_slice()).unwrap(),&mut cmd_backing);
		
		match command {
		    Ok(cmd) => {
			cmd.execute();
		    },
		    Err(Error::TooManyArgs) => {
			kprintln!("");
			kprint!("too many arguments, max 64", );
		    },
		    Err(Error::Empty) => {
			// do nothing
		    },
		}
		
		// -> path and args -> execute
		buf = StackVec::new(&mut buff_backing);
		kprintln!("");
		kprint!("{}", prefix);
	    },

	    byte if (byte == BACKSPACE || byte == DELETE) => {
		match buf.pop() {
		    Some(_some) => {console.write(&[BACKSPACE, b' ', BACKSPACE]).expect("backspace/del shell character");},
		    None => {console.write_byte(BELL);},
		}
	    },
	    
	    byte if (byte < 32) => { // non-printable chars
		console.write_byte(BELL);
	    },
		
	    _ => {
		match buf.push(new_byte) {
		    Ok(_ok) => {kprint!("{}", new_byte as char);},
		    Err(_err) => {console.write_byte(BELL);},
		}
	    }
	    

	}
	// read input...
	// if backspace char remove last buffer entry...
	// append to buffer
	// if return parse?
	
	
    }
}


