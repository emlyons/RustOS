use stack_vec::StackVec;

use crate::console::{kprint, kprintln, CONSOLE};

use shim::io::Write;

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
        unimplemented!()
    }
}

const NEWLINE: u8 = 10;
const RETURN: u8 = 13;
const BACKSPACE: u8 = 08;
const DELETE: u8 = 127;
const BELL: u8 = 7;

/*



    [] implement the echo built-in: echo $a $b $c should print $a $b $c

    [X] accept both \r and \n as “enter”, marking the end of a line

    [X] accept both backspace and delete (ASCII 8 and 127) to erase a single character

    [X] ring the bell (ASCII 7) if an unrecognized non-visible character is sent to it

    [] print unknown command: $command for an unknown command $command

    [X] disallow backspacing through the prefix

    [X] disallow typing more characters than allowed

    [X] accept commands at most 512 bytes in length

    [] accept at most 64 arguments per command

    [] start a new line, without error, with the prefix if the user enters an empty command

    [] print error: too many arguments if the user passes in too many arguments


*/
/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell(prefix: &str) -> ! {

    let mut buff_backing = [0u8; 512];
    let mut buf = StackVec::new(&mut buff_backing);
    let mut index: usize = 0;

    kprint!("{}", prefix);
    
    loop {
	// print out prefix
	let mut console = CONSOLE.lock();
	let new_byte = console.read_byte();

	match new_byte {
	    
	    byte if (byte == NEWLINE || byte == RETURN) => {
		// parse command -> path and args -> execute
		buf = StackVec::new(&mut buff_backing);
		kprintln!("");
		kprint!("{}", prefix);
	    },

	    byte if (byte == BACKSPACE || byte == DELETE) => {
		match buf.pop() {
		    Some(some) => {console.write(&[BACKSPACE, b' ', BACKSPACE]);},
		    None => {console.write_byte(BELL);},
		}
	    },
	    
	    byte if (byte < 32) => { // non-printable chars
		console.write_byte(BELL);
	    },
		
	    _ => {
		match buf.push(new_byte) {
		    Ok(ok) => {kprint!("{}", new_byte as char);},
		    Err(err) => {console.write_byte(BELL);},
		}
	    }
	    

	}
	// read input...
	// if backspace char remove last buffer entry...
	// append to buffer
	// if return parse?
	
	
    }
}


