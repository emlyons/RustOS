# rust-os-cs140e
## An Experimental Course on Operating Systems

Assignments from the [CS140 course](https://cs140e.sergio.bz/).

### Directory Structure

```
.
├── bin : common binaries/utilities
├── doc : reference documents
├── ext : external files (e.g., resources for testing)
├── tut : tutorial/practices
│    ├── 0-rustlings
│    ├── 1-blinky
│    ├── 2-shell
│    ├── 3-fs
│    ├── 4-spawn
│    └── 5-multicore : questions for lab5 *
├── boot : bootloader
├── kern : the main os kernel *
├── lib  : required libraries
│     ├── aarch *
│     ├── kernel_api *
│     ├── fat32
│     ├── pi *
│     ├── shim
│     ├── stack-vec
│     ├── ttywrite
│     ├── volatile
│     └── xmodem
└── user : user level program *
      ├── fib
      ├── sleep
      └── socket *
```

### Rust Versioning
```
$ rustup install nightly-2018-01-09
$ rustup default nightly-2018-01-09
$ rustup override set nightly-2018-01-09
$ rustup component add rust-src

$ cargo install xargo --version 0.3.10

$ rustc --version
rustc 1.25.0-nightly (b5392f545 2018-01-08)

$ xargo --version
xargo 0.3.10
cargo 0.25.0-nightly (a88fbace4 2017-12-29)
```

## Bootstrapping Raspberry Pi
Phase 0 - 4 from [Assignment 0: Blinky](https://cs140e.sergio.bz/assignments/0-blinky/).
Get the enviornment setup and make and LED blink in C and Rust.

### Phase 0: Getting Started
- [x] Getting your Pi Ready
- [x] Getting the Skeleton Code

### Phase 1: Baking Pi
- [x] Installing Driver
- [x] Powering the Pi
- [x] Running Programs

### Phase 2: LED There Be Light
- [x] GPIO: General Purpose I/O
- [x] Testing the LED

### Phase 3: Shining C
- [x] Installing a Cross-Compiler
- [x] Talking to Hardware
- [x] GPIO Memory-Mapped Interface
- [x] Writing the Code

### Phase 4: Rusting Away
- [x] Installing Rust and Xargo
- [x] Writing the Code


## Shell and Bootloader
Phase 0 - 2 from [Assignment 1: Shell](https://cs140e.sergio.bz/assignments/1-shell/).
Write `stack-vec`, `volatile`, `ttywrite`, and `xmodem` libraries.

### Phase 0: Getting Started
- [x] Getting the Skeleton Code

### Phase 2: Oxidation
- [x] Subphase A: StackVec
- [x] Subphase B: volatile
- [x] Subphase C: xmodem
- [x] Subphase D: ttywrite

### Phase 3: *Not* a Seashell
- [x] Subphase A: Getting Started
- [x] Subphase B: System Timer
- [x] Subphase C: GPIO
- [x] Subphase D: UART
- [x] Subphase E: The Shell
     
### Phase 4: Boot 'em Up
- [x] Loading Binaries
- [x] Making Space
- [x] Implementing the Bootloader


## FAT32 Filesystem
Phase 0 - 4 from [Assignment 2: File System](https://cs140e.sergio.bz/assignments/2-fs/).

### Phase 0: Getting Started
- [x] Getting the Skeleton Code

### Phase 1: Memory Lane
- [x] Subphase A: Panic!
- [x] Subphase B: ATAGS
- [x] Subphase C: Warming Up
- [x] Subphase D: Bump Allocator
- [x] Subphase E: Bin Allocator

### Phase 2: 32-bit Lipids
- [x] Implementation

### Phase 3: Saddle Up
- [x] Subphase A: SD Driver FFI
- [x] Subphase B: File System

### Phase 4: Mo'sh
- [x] Working Directory
- [x] Commands
- [x] Implementation


## Preemptive Multitasking
Phase 0 - 4 from [Assignment 2: Spawn](https://cs140e.sergio.bz/assignments/3-spawn/).

### Phase 0: Getting Started
- [x] Getting the Skeleton Code

### Phase 1: ARM and a Leg
- [x] Subphase A: ARMv8 Overview
- [x] Subphase B: Instructions
- [x] Subphase C: Switching to EL1
- [x] Subphase D: Exception Vectors
- [x] Subphase E: Exception Return

### Phase 2: It's a Process
- [x] Subphase A: Processes
- [x] Subphase B: The First Process
- [x] Subphase C: Timer Interrupts
- [x] Subphase D: Scheduler
- [x] Subphase E: Sleep

### Phase 3: Memory Management Unit
- [x] Subphase A: Virtual Memory
- [x] Subphase B: Page Table

### Phase 4: Programs In The Disk
- [x] Subphase A: Load A Program
- [x] Subphase B: User Processes


## Multicore and Networking
Phase 0 - 3 from [Assignment 2: Spawn]https://tc.gts3.org/cs3210/2020/spring/lab/lab5.html).

### Phase 0: Getting Started
- [ ] Getting the Skeleton Code

### Phase 1: Enabling Multicore
- [ ] Subphase A: Waking Up Other Cores
- [ ] Subphase B: Mutex, Revisited
- [ ] Subphase C: Multicore Scheduling

### Phase 2: TCP Networking
- [ ] Subphase A: Networking 101
- [ ] Subphase B: Network Driver
- [ ] Subphase C: Process Resource Management
- [ ] Subphase D: Socket System Calls

### Phase 3: Echo Server
- [ ] Implementation 