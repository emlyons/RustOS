use shim::path::{Path, PathBuf, Component};

use stack_vec::StackVec;
use alloc::vec::Vec;

use pi::atags::Atags;
use pi::interrupt::{Controller, Interrupt};

use fat32::traits::FileSystem;
use fat32::traits::{Dir, File, Entry};

use kernel_api::syscall;

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

use shim::io::{Read, Write};
use core::str;
use pi::gpio;

use core::str::FromStr;
use core::time::Duration;

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
    active: bool,
}

impl Shell {
    fn new() -> Self {
	let dir = PathBuf::from(r"/");
	Shell{pwd: dir, active: true}
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

    fn pop(&mut self) {
	if self.pwd.pop() == false {
	    self.root();
	}
    }

    fn root(&mut self) {
	while self.pwd.pop() {};
	self.pwd.push("/");
    } 
    
    fn new_line(&self, prefix: &str) {
	kprint!("\n({}) {} ", self.pwd.as_path().display(), prefix);
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
	"ls" => list_directory(cmd, shell),
	"pwd" => print_directory(shell),
	"cat" => concatenate_file(cmd, shell),
	"exit" => exit(shell),
	"sleep" => sleep(cmd),
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
    if (cmd.args.len() != 2) {
	kprint!("\ninvalid argument");
	return;
    }

    if let Ok(val) = u8::from_str(cmd.args[1]) {    
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
    if (cmd.args.len() != 2) {
	return;
    }
    
    if !shell.change_pwd(&cmd.args[1]) {
	kprint!("\n{}: {}: No such file or directory", cmd.args[0], cmd.args[1]);
    }
}

fn list_directory(cmd: &Command, shell: &mut Shell) {
    use fat32::traits::{Metadata, Timestamp};
    let mut hidden = false;
    let mut path = shell.pwd.clone();
    
    if cmd.args.len() == 3 && cmd.args[1] == "-a" {
	hidden = true;
	path.push(cmd.args[2]);
    }
    else if cmd.args.len() == 2 && cmd.args[1] == "-a"{
	hidden = true;
    }
    else if cmd.args.len() == 2 {
	path.push(cmd.args[1]);
    }

    if let Ok(entry) = FILESYSTEM.open(path.as_path()) {
	if let Some(dir) = entry.as_dir() {
	    for entry in dir.entries().unwrap() {
		if !entry.metadata().hidden() || hidden {
		    kprintln!("");
		    
		    match entry.metadata().read_only() {
			true => {kprint!("r");},
			false => {kprint!("w");},
		    }
		    
		    match entry.metadata().hidden() {
			true => {kprint!("h");},
			false => {kprint!("-");},
		    }
		    
		    match entry.metadata().system() {
			true => {kprint!("s");},
			false => {kprint!("-");},
		    }
		    
		    match entry.metadata().directory() {
			true => {kprint!("d");},
			false => {kprint!("f");},
		    }
		    
		    match entry.metadata().archive() {
			true => {kprint!("a");},
			false => {kprint!("-");},
		    }
		    
		    kprint!(" {:02}/{:02}/{:04} {:02}:{:02}:{:02} ", entry.metadata().created().day(), entry.metadata().created().month(), entry.metadata().created().year(), entry.metadata().created().hour(), entry.metadata().created().minute(), entry.metadata().created().second());
		
		    kprint!("{:02}/{:02}/{:04} {:02}:{:02}:{:02} ", entry.metadata().modified().day(), entry.metadata().modified().month(), entry.metadata().modified().year(), entry.metadata().modified().hour(), entry.metadata().modified().minute(), entry.metadata().modified().second());
		
		    kprint!(" {:10} {}", entry.metadata().file_size(), entry.name());
		}
	    }
	}
    }
    else {
	kprint!("\n{}: No such directory", cmd.args[0]);
    }
}

fn print_directory(shell: &mut Shell) {
    kprint!("\n{}", shell.pwd.as_path().display());
}

fn concatenate_file(cmd: &Command, shell: &mut Shell) {
    assert_eq!(cmd.args[0], "cat");
    if (cmd.args.len() != 2) {
	kprint!("\ninvalid argument");
	return;
    }
    
    let mut file_path = shell.pwd.clone();
    file_path.push(Path::new(cmd.args[1]));
	
    if let Ok(entry) = FILESYSTEM.open(file_path.as_path()) {
	if let Some(mut file) = entry.into_file() {
	    kprintln!("");
	    let mut read_bytes = 0;
	    let mut data = [0u8; 1024];
	    while read_bytes < file.size() {
		if let Ok(bytes_returned) = file.read(&mut data) {
		    if let Ok(text) = str::from_utf8(&data[0..bytes_returned]) {
			kprint!("{:?}", text);
		    }
		    read_bytes += bytes_returned as u64;
		}
		else {
		    return;
		}
	    }
	    return;
	}
    }
    kprint!("\n{}: {}: No such file", cmd.args[0], cmd.args[1]);
}

fn exit(shell: &mut Shell) {
    shell.active = false;
}

fn sleep(cmd: &Command) {   
    assert_eq!(cmd.args[0], "sleep");
    if (cmd.args.len() != 2) {
	kprint!("\ninvalid argument");
	return;
    }
    
    if let Ok(ms) = u64::from_str(cmd.args[1]) {
	let dur = Duration::from_millis(ms);
	if let Ok(duration) = syscall::sleep(dur) {
	    kprint!("\nslept for {} milliseconds", duration.as_millis());
	} else {
	    kprint!("\nan error occurred");
	}
    }
}

// TODO: THIS IS FOR DEBUGGING AND SHOULD NOT REMAIN
fn panic() {
    unreachable!();
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) {

    let mut session = Shell::new();
    let mut buff_backing = [0u8; 512];
    let mut buf = StackVec::new(&mut buff_backing);

    session.new_line(prefix);
    
    loop {
	let mut console = CONSOLE.lock();
	let new_byte = console.read_byte();

	match new_byte {

	    // current command line entered as command
	    byte if (byte == NEWLINE || byte == RETURN) => {
		let mut cmd_backing: [&str; 64] = [""; 64];
		let command = Command::parse(str::from_utf8(buf.as_slice()).unwrap(),&mut cmd_backing);		
		match command {
		    Ok(cmd) => {
			execute(&cmd, &mut session);
			if !session.active {
			    break;
			}
		    },
		    Err(Error::TooManyArgs) => {
			kprint!("\ntoo many arguments, max 64", );
		    },
		    Err(Error::Empty) => {
			// do nothing
		    },
		}
		buf = StackVec::new(&mut buff_backing);
		session.new_line(prefix);
	    },

	    // remove chars from command line
	    byte if (byte == BACKSPACE || byte == DELETE) => {
		match buf.pop() {
		    Some(_some) => {console.write(&[BACKSPACE, b' ', BACKSPACE]).expect("backspace/del shell character");},
		    None => {console.write_byte(BELL);},
		}
	    },

	    // non printable char enteered to command line
	    byte if (byte < 32) => {
		console.write_byte(BELL);
	    },

	    // valid char input
	    _ => {
		match buf.push(new_byte) {
		    Ok(_ok) => {kprint!("{}", new_byte as char);},
		    Err(_err) => {console.write_byte(BELL);},
		}
	    }
	}	
    }
}


