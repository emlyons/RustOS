use shim::path::{Path, PathBuf, Component};

use stack_vec::StackVec;
use alloc::vec::Vec;

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, File, Entry};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

use shim::io::{Read, Write};
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

/// shell session state
struct Shell {
    pwd: PathBuf,
}

impl Shell {
    fn new() -> Self {
	let dir = PathBuf::from(r"/");
	Shell{pwd: dir}
    }

    fn change_pwd(&mut self, path: &str) -> bool {
	let curr_pwd = self.pwd.clone();
	let p = Path::new(path);
	
	for entry in p.components() {
	    match entry {
		Component::CurDir => continue,
		Component::ParentDir => {self.pop();},
		Component::RootDir => {self.root();},
		Component::Normal(name) => {
		    self.pwd.push(name);
		    if FILESYSTEM.open(self.pwd.as_path()).is_err() { // or is file
			self.pwd = curr_pwd.clone();
			return false;
		    }
		},
		_ => unreachable!(),
	    };
	}
	true
    }

    fn list_pwd(&mut self) {
	if let Ok(entry) = FILESYSTEM.open(self.pwd.as_path()) {
	    if let Some(dir) = entry.as_dir() {
		kprintln!("");
		for entry in dir.entries().unwrap() {
		    // attr?, date, 
		    kprintln!("{:?} {}", entry.metadata(), entry.name());
		}
	    }
	}
    }

    fn concatenate_file(&mut self, name: &str) {
	let mut file_path = self.pwd.clone();
	file_path.push(Path::new(name));
	
	if let Ok(entry) = FILESYSTEM.open(file_path.as_path()) {
	    if let Some(mut file) = entry.into_file() {
		
		let mut read_bytes = 0;
		let mut data = [0u8; 512];
		while read_bytes < file.size() {
		    if let Ok(bytes_returned) = file.read(&mut data) {
			kprintln!("{:?}", &data[0..bytes_returned]);
			read_bytes += bytes_returned as u64;
		    }
		    else {
			return;
		    }
		}
	    }
	}
    }

    fn pop(&mut self) {
	if self.pwd.pop() == false {
	    self.root();
	}
    }

    fn root(&mut self) {
	while self.pwd.pop() {};
	self.pwd.push("/");
    } 
    
    fn new_line(&self) {
	kprint!("\n({}) > ", self.pwd.as_path().display());
    }
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
}

/// fullfills command request if present/valid in Command struct
fn execute(cmd: &Command, shell: &mut Shell) {
    match cmd.path() {
	"echo" => echo(cmd),
	"panic" => panic(),
	"binled" => binary_led(cmd),
	"cd" => change_directory(cmd, shell),
	"ls" => list_directory(shell),
	"pwd" => print_directory(shell),
	"cat" => concatenate_file(cmd, shell),
	_ => {
	    kprint!("\nunknown command");
	},
    }
}

fn echo (cmd: &Command) {
    assert_eq!(cmd.args[0], "echo");
    if (cmd.args.len() > 1) {
	kprintln!("");
	cmd.args.as_slice().iter().skip(1).for_each(|arg| kprint!("{} ", arg));
    }
}

fn binary_led(cmd: &Command) {
    assert_eq!(cmd.args[0], "binled");
    if (cmd.args.len() == 2) {
	// parse number from string, if unsuccessful turn off leds
	let mut val: u8 = 0;
	let arg = cmd.args[1].parse::<u8>();
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

fn change_directory(cmd: &Command, shell: &mut Shell) {
    assert_eq!(cmd.args[0], "cd");
    if (cmd.args.len() == 2) {
	if !shell.change_pwd(&cmd.args[1]) {
	    kprint!("\n{}: {}: No such file or directory", cmd.args[0], cmd.args[1]);
	}
    }
}

fn list_directory(shell: &mut Shell) {
    kprint!("\nTODO: list directory");
}

fn print_directory(shell: &mut Shell) {
    kprint!("\n{}", shell.pwd.as_path().display());
}

fn concatenate_file(cmd: &Command, shell: &mut Shell) {
    assert_eq!(cmd.args[0], "cat");
    if (cmd.args.len() == 2) {
	kprint!("\nTODO: concatenate file: {}{}", shell.pwd.as_path().display(), cmd.args[1]);
    }
}

// TODO: THIS IS FOR DEBUGGING AND SHOULD NOT REMAIN
fn panic() {
    unreachable!();
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) -> ! {

    let mut session = Shell::new();
    let mut buff_backing = [0u8; 512];
    let mut buf = StackVec::new(&mut buff_backing);

    session.new_line();
    
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
			execute(&cmd, &mut session);
		    },
		    Err(Error::TooManyArgs) => {
			kprint!("\ntoo many arguments, max 64", );
		    },
		    Err(Error::Empty) => {
			// do nothing
		    },
		}
		
		// -> path and args -> execute
		buf = StackVec::new(&mut buff_backing);
		session.new_line();
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
    }
}


