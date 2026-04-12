use std::{alloc::{Layout, alloc}, ffi::c_void, sync::atomic::{AtomicBool, AtomicUsize, Ordering}, thread::sleep, vec};

use libc::{iovec, process_vm_readv};
use procfs::{self, process::Process};
use rand::{RngExt}; 
static DELAYED_READS_ENABLED: AtomicBool = AtomicBool::new(false);
static DELAYED_READ_MIN: AtomicUsize = AtomicUsize::new(5);
static DELAYED_READ_MAX: AtomicUsize = AtomicUsize::new(20);

pub fn random(start: usize, end: usize) -> usize
{
    let mut range = rand::rng(); // get a new range
    return range.random_range(start..=end); // return random from start to end
}
#[allow(non_camel_case_types)]
pub struct a_process
{
    pub process: Option<Process>, // only way to put it as None
    pub process_id: u32, // process id
    pub file_name: String, // the name of the process, eg "cs2.exe"
    pub cmd_line: Vec<String> // command line arguments that the process was created with
}
#[allow(non_camel_case_types)]
pub struct a_module {
    pub name: String,      // e.g. "client.dll"
    pub base: u64,         // base address
    pub size: u64,         // size of the module
    
}
pub fn find_processes() -> Vec<a_process> // returns a list of processes
{
    let mut processes : Vec<a_process> = vec![]; // make vec
    for proc in procfs::process::all_processes().unwrap() // go through each process via procfs
    {
        // sanity checks
        let Ok(process) = proc else { continue; }; // check if the process is valid
        let Ok(exe_path) = process.exe() else { continue; }; // check if it has a valid exe
        let Ok(cmdline) = process.cmdline() else { continue; }; // check if it has a command line

        let file_name = exe_path // im not going to attempt to explain it, TODO: explain this shit
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let process_id: u32 = process.pid() as u32; // get its process id
        processes.push(a_process { process: Some(process), process_id, file_name, cmd_line: cmdline }); // push a new process struct

    }
    return processes;
}
pub fn find_process_from_process_id(target_process_id: u32) -> a_process
{

    for current_process in find_processes() // get all processes
    {
        if current_process.process_id == target_process_id // check its pid
        {
            return current_process; // return the a_process
        }
    }
    return a_process { process: None, process_id: 0, file_name: String::from(""), cmd_line:  vec![]}; // return a_process with nothing
}
pub fn find_process_from_name(target_process_name: String) -> a_process
{
    for process in find_processes() // get all processes
    {
        if process.file_name.contains(target_process_name.as_str()) // check its pid
        {
            return process; // return the a_process
        }
    }
    return a_process { process: None, process_id: 0, file_name: String::from(""), cmd_line: vec![] }; // return a_process with nothing
}
pub fn find_module_from_name(target_module_name: String, target_process: &a_process) -> a_module {
    let process = match &target_process.process { // check if a_process is even valid
        Some(p) => p, // process = Process if valid
        None => return a_module {  // return a_module with nothing
            name: String::from(""), 
            base: 0x0, 
            size: 0x0 
        },
    };

    let Ok(maps) = process.maps() else { // see if its mappings is valid
        return a_module {  // return a_module if not valid
            name: String::from(""), 
            base: 0x0, 
            size: 0x0 
        };
    };

    let lower_target = target_module_name.to_lowercase(); // convert name to lowercase

    for map in maps { // go through each map
        let pathname = match &map.pathname { // see if it has a valid path name
            procfs::process::MMapPath::Path(path) => path, // return its path
            _ => continue, // continue
        };

        let file_name_lower = pathname.file_name() // get its file name then converts to lowercase TODO: explain this shit
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if file_name_lower.contains(&lower_target) || file_name_lower == lower_target { //  see if the file name has the target name in it
            let (start_addr, end_addr) = map.address; // get start and end addr
            let real_name = pathname.file_name() // get its exe / file name no lowercase
                .and_then(|s| s.to_str())
                .unwrap_or("unknown.so")
                .to_string();

            return a_module { // return a_module
                name: real_name,
                base: start_addr,
                size: end_addr - start_addr,
            };
        }
    }

    a_module { // return empty a_module if nothing
        name: String::from(""),
        base: 0x0,
        size: 0x0,
    }
}
pub fn enable_read_sleep()
{
    DELAYED_READS_ENABLED.store(true, Ordering::Relaxed); // enable read sleep
}
pub fn disable_read_sleep()
{
    DELAYED_READS_ENABLED.store(false, Ordering::Relaxed) // disable read sleep
}
pub fn set_read_min_sleep(delay: usize) // miliseconds
{
    DELAYED_READ_MIN.store(delay, Ordering::Relaxed) // set read minimum sleep
}
pub fn set_read_max_sleep(delay: usize) // miliseconds
{
    DELAYED_READ_MAX.store(delay, Ordering::Relaxed) // set read maximum sleep
}
pub fn set_delay_range(min_ms: usize, max_ms: usize) { // miliseconds
    DELAYED_READ_MIN.store(min_ms, Ordering::Relaxed); // set minimum sleep
    DELAYED_READ_MAX.store(max_ms, Ordering::Relaxed); // set maximum sleep
}
pub fn read_bytes(target_process: &a_process, target_address: u64, size: usize) -> Vec<u8> // read a list of bytes
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
        // sleep a random amount of miliseconds from the delayed read minimuim and maximum
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let mut buffer: Vec<u8> = vec![];  // create a new byte array
    let local_iov = iovec { // create the local iov (buffer)
        iov_base: buffer.as_mut_ptr() as *mut c_void, // set base as buffer ptr
        iov_len: size // set the size to the passed size parameter
    };
    let remote_iov = iovec { // create the remote iov (target)
        iov_base: target_address as *mut c_void, // target address
        iov_len: size // size of the passed size parameter
    };
    let result_read: usize = unsafe { // read memory and get the size of the read bytes as usize
        process_vm_readv(target_process.process_id as i32, // read process memory with the a_process process_id param
            &local_iov as *const iovec, 1,  // get local iov with 1 element ^^^
            &remote_iov as *const iovec, 1, // get remote iov with 1 element ^^^
            0 // flag 0 for unused
        ) as usize // convert isize to usize
    };
    if result_read == size // if the read size was the same size as the passed size parameter
    {
        return buffer; // return the byte array
    }
    return vec![]; // return a empty byte array
}
pub fn read_u64(target_process: &a_process, target_address: u64) -> u64
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
        // sleep a random amount of miliseconds from the delayed read minimuim and maximum
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let buffer: Vec<u8> = read_bytes(target_process, // read the bytes and get the buffer with the target process
        target_address, // target address (u64)
        std::mem::size_of::<u64>() // the size of a u64
    ); 
    if buffer.len() < std::mem::size_of::<u64>() // if the amount of bytes in buffer is below
    {
        return 0x0; // return nothing
    }
    let read: u64 = u64::from_ne_bytes(buffer.try_into().unwrap()); // convert the byte array to u64
    return read // return the actual read
}
pub fn read_u32(target_process: &a_process, target_address: u64) -> u32
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let buffer: Vec<u8> = read_bytes(target_process, // read the bytes and get the buffer with the target process
        target_address, // target address (u64)
        std::mem::size_of::<u32>() // the size of a u32
    ); 
    if buffer.len() < std::mem::size_of::<u32>() // if the amount of bytes in buffer is below
    {
        return 0x0; // return nothing
    }
    let read: u32 = u32::from_ne_bytes(buffer.try_into().unwrap()); // convert the byte array to u32
    return read // return the actual read
}
pub fn read_u16(target_process: &a_process, target_address: u64) -> u16 
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let buffer: Vec<u8> = read_bytes(target_process, // read the bytes and get the buffer with the target process
        target_address, // target address (u64)
        std::mem::size_of::<u16>() // the size of a u16
    ); 
    if buffer.len() < std::mem::size_of::<u16>() // if the amount of bytes in buffer is below
    {
        return 0x0; // return nothing
    }
    let read: u16 = u16::from_ne_bytes(buffer.try_into().unwrap()); // convert the byte array to u16
    return read // return the actual read
}
pub fn read_u8(target_process: &a_process, target_address: u64) -> u8
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let buffer: Vec<u8> = read_bytes(target_process, // read the bytes and get the buffer with the target process
        target_address, // target address (u64)
        std::mem::size_of::<u8>() // the size of a u8
    ); 
    if buffer.len() < std::mem::size_of::<u8>() // if the amount of bytes in buffer is below
    {
        return 0x0; // return nothing
    }
    let read: u8 = u8::from_ne_bytes(buffer.try_into().unwrap()); // convert the byte array to u8
    return read // return the actual read
}
pub fn read_usize(target_process: &a_process, target_address: u64) -> usize
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let buffer: Vec<u8> = read_bytes(target_process, // read the bytes and get the buffer with the target process
        target_address, // target address (u64)
        std::mem::size_of::<usize>() // the size of a usize
    ); 
    if buffer.len() < std::mem::size_of::<usize>() // if the amount of bytes in buffer is below
    {
        return 0x0; // return nothing
    }
    let read: usize = usize::from_ne_bytes(buffer.try_into().unwrap()); // convert the byte array to usize
    return read // return the actual read
}

