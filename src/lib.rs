
use std::{ffi::c_void, fs::exists, mem::MaybeUninit, sync::atomic::{AtomicBool, AtomicUsize, Ordering}, thread::sleep, vec};

use libc::{iovec, process_vm_readv, process_vm_writev};
use procfs::{self, process::Process};
use rand::{RngExt}; 
static DELAYED_READS_ENABLED: AtomicBool = AtomicBool::new(false);
static DELAYED_READ_MIN: AtomicUsize = AtomicUsize::new(5);
static DELAYED_READ_MAX: AtomicUsize = AtomicUsize::new(20);
#[derive(Debug, PartialEq, Eq)]
pub enum MemError {
    Unsupported,
    InvalidProcess,
    InvalidAddress,
    InvalidMap,
    InvalidSize,
    InvalidBuffer,
    ReadFailure,
    Syscall(i32),
    PartialRead {
        expected: usize,
        actual: usize,
    },
    PartialWrite {
        expected: usize,
        actual: usize,
    },
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProcError {
    InvalidProcess,
    InvalidPath,
    InvalidExe,
    InvalidCmdLine,
    ProcessNotFound,
    
}
#[derive(Debug, PartialEq, Eq)]
pub enum MapError {
    InvalidProcess,
    InvalidMaps,
    InvalidPath,
    MapNotFound,
}


pub fn random(start: usize, end: usize) -> usize
{
    let low = start.min(end);
    let hi = start.max(end);
    let mut range = rand::rng(); // get a new range
    return range.random_range(low..=hi); // return random from start to end
}
#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub struct a_process
{
    pub process_id: i32, // process id
    pub file_name: String, // the name of the process, eg "cs2.exe"
    pub cmd_line: Vec<String> // command line arguments that the process was created with
}
impl a_process {
    pub fn new(pid: i32) -> Result<a_process, ProcError> {
        let proc = match Process::new(pid) {
            Ok(p) => p,
            Err(_e) => {
                return Err(ProcError::InvalidProcess);
            }
        };
        let exe = match proc.exe().ok() {
            Some(e) => e,
            None => {
                return Err(ProcError::InvalidPath);
            }
        };
        let cmdline: Vec<String> = match proc.cmdline().ok() {
            Some(c) => c,
            None => vec![]
        };
        let name = match exe.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => "[unknown process]".to_string()
        };
        
        Ok(a_process { process_id: pid, file_name: name, cmd_line: cmdline })
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub struct a_module {
    pub name: String,      // e.g. "client.dll"
    pub base: usize,         // base address
    pub size: usize,         // size of the module
    
}
pub fn process_alive(process_id: i32) -> bool 
{
    let path_str = "/proc/".to_owned() + &process_id.to_string(); // get the path of proc
    return exists(path_str).unwrap_or(false); // return if the process folder exists
}
pub fn find_processes() -> Vec<a_process> // returns a list of processes
{
    let mut processes : Vec<a_process> = vec![]; // make vec
    for proc in procfs::process::all_processes().unwrap() // go through each process via procfs
    {
        // sanity checks
        let Ok(process) = proc else { continue; }; // check if the process is valid
        let Ok(exe_path) = process.exe() else { continue; }; // check if it has a valid exe
        let cmdline: Vec<String> = match process.cmdline() { Ok(c) => c, Err(_e) => vec![] }; // check if it has a command line
        let file_name = exe_path // im not going to attempt to explain it, TODO: explain this shit
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let process_id: i32 = process.pid(); // get its process id
        processes.push(a_process { process_id, file_name, cmd_line: cmdline }); // push a new process struct

    }
    return processes;
}
pub fn find_modules(process: &a_process) -> Vec<a_module> {
    let process = match Process::new(process.process_id) {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    let Ok(maps) = process.maps() else {
        return vec![];
    };

    let mut modules: Vec<a_module> = vec![];

    for map in maps {
        let (start_addr, end_addr) = map.address;

        let name = match &map.pathname {
            procfs::process::MMapPath::Path(path) => {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            }
            _ => continue
        };
        

        modules.push(a_module {
            name,
            base: start_addr as usize,
            size: end_addr as usize - start_addr as usize,
        });
    }

    modules
}
pub fn find_maps(process: &a_process) -> Result<Vec<a_module>, MapError> {
    let process = match Process::new(process.process_id) {
        Ok(p) => p,
        Err(_) => return Err(MapError::InvalidProcess),
    };

    let Ok(memmaps) = process.maps() else {
        return Err(MapError::InvalidMaps);
    };

    let mut maps: Vec<a_module> = vec![];

    for map in memmaps {
        let (start_addr, end_addr) = map.address;

        let name = match &map.pathname {
            procfs::process::MMapPath::Path(path) => {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            }

            procfs::process::MMapPath::Heap => "[heap]".to_string(),
            procfs::process::MMapPath::Stack => "[stack]".to_string(),

            procfs::process::MMapPath::TStack(tid) => {
                format!("[thread stack {}]", tid)
            }

            procfs::process::MMapPath::Vdso => "[vdso]".to_string(),
            procfs::process::MMapPath::Vvar => "[vvar]".to_string(),
            procfs::process::MMapPath::Vsyscall => "[vsyscall]".to_string(),

            procfs::process::MMapPath::Anonymous => "[anonymous]".to_string(),

            other => format!("{:?}", other),
        };

        maps.push(a_module {
            name,
            base: start_addr as usize,
            size: end_addr as usize - start_addr as usize,
        });
    }

    Ok(maps)
}
pub fn find_process_from_process_id(target_process_id: i32) -> Result<a_process, ProcError>
{
    return a_process::new(target_process_id);
}
pub fn find_process_from_name(target_process_name: String) -> Result<a_process, ProcError>
{
    for process in find_processes() // get all processes
    {
        if process.file_name.contains(target_process_name.as_str()) // check its pid
        {
            return Ok(process); // return the a_process
        }
    }
    return Err(ProcError::ProcessNotFound);
}
pub fn find_module_from_name(target_module_name: String, target_process: &a_process, case_insensitive: bool) -> Result<a_module, MapError> {
    for module in find_modules(&target_process) 
    {
        if case_insensitive {
            if module.name.to_lowercase().contains(&target_module_name.to_lowercase()) {
                return Ok(module);
            }
        } else {
            if module.name == target_module_name {
                return Ok(module);
            }
        }
    }
    return Err(MapError::MapNotFound);
}
pub fn find_map_from_name(target_map_name: String, target_process: &a_process, case_insensitive: bool) -> Result<a_module, MapError> {
    let maps = match find_maps(&target_process) {
        Ok(m) => m,
        Err(e) => return Err(e)
    };
    for map in maps
    {
        if case_insensitive {
            if map.name.to_lowercase().contains(&target_map_name.to_lowercase()) {
                return Ok(map);
            }
        } else {
            if map.name == target_map_name {
                return Ok(map);
            }
        }
    }
    return Err(MapError::MapNotFound);
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
pub fn read_bytes(target_process: &a_process, target_address: usize, size: usize) -> Result<Vec<u8>, MemError> // read a list of bytes
{
    if size == 0 {
        return Ok(vec![]);
    }
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed) // if delayed reading is on
	{
        // sleep a random amount of miliseconds from the delayed read minimuim and maximum
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    let mut buffer: Vec<u8> = vec![0u8; size];  // create a new byte array
    let local_iov = iovec { // create the local iov (buffer)
        iov_base: buffer.as_mut_ptr() as *mut c_void, // set base as buffer ptr
        iov_len: size // set the size to the passed size parameter
    };
    let remote_iov = iovec { // create the remote iov (target)
        iov_base: target_address as *mut c_void, // target address
        iov_len: size // size of the passed size parameter
    };
    let result_read: isize = unsafe { // read memory and get the size of the read bytes as usize
        process_vm_readv(target_process.process_id as i32, // read process memory with the a_process process_id param
            &local_iov as *const iovec, 1,  // get local iov with 1 element ^^^
            &remote_iov as *const iovec, 1, // get remote iov with 1 element ^^^
            0 // flag 0 for unused
        ) 
    };
    if result_read > 0 // if the read size is above 0
    {
        if result_read == size as isize {
            return Ok(buffer); // return the byte array
        } else {
            return Err(MemError::PartialRead { expected: size, actual: result_read as usize });
        }
    } else {
        return Err(
            MemError::Syscall(match std::io::Error::last_os_error().raw_os_error() { Some(e) => e, None => 935 })
        ) // return a empty byte array

    }
}
pub fn write_bytes(target_process: &a_process, target_address: usize, buffer: &[u8]) -> Result<usize, MemError> {

    if buffer.is_empty() {
        return Err(MemError::InvalidBuffer);
    }
    if target_process.process_id == 0 || !process_alive(target_process.process_id) {
        return Err(MemError::InvalidProcess);
    }
    if target_address <= 0 {
        return Err(MemError::InvalidAddress);
    }
    let local_iov = iovec { // create the local iov (buffer)
        iov_base: buffer.as_ptr() as *mut c_void, // set base as buffer ptr
        iov_len: buffer.len() // set the size to the passed size parameter
    };
    let remote_iov = iovec { // create the remote iov (target)
        iov_base: target_address as *mut c_void, // target address
        iov_len: buffer.len() // size of the passed size parameter
    };
    let result_write = unsafe {
        process_vm_writev(target_process.process_id as i32, &local_iov, 1, &remote_iov, 1, 0)
    };
    if result_write < 0 {
        return Err(MemError::Syscall(match std::io::Error::last_os_error().raw_os_error() { Some(e) => e, None => 935 }));
    }
    let actual = result_write as usize;
    if actual != buffer.len() {
        return Err(MemError::PartialWrite { expected: buffer.len(), actual: result_write as usize });
    } else {
        return Ok(result_write as usize);
    }
}
pub fn read<T: Copy + 'static>(target_process: &a_process, target_address: usize) -> Result<T, MemError>
{
    // read however many bytes of the type they want to read with
    let read_bytes = match read_bytes(target_process, target_address, std::mem::size_of::<T>()) {
        Ok(r) => r,
        Err(e) => {
            return Err(e);
        }
    };
    // this might be changed to return partial reads aswell
    if read_bytes.len() != std::mem::size_of::<T>() // if we didnt read the right amount of bytes
    {
        return Err(MemError::PartialRead { expected: std::mem::size_of::<T>(), actual: read_bytes.len() }); // return none
    } else {
        let mut out = MaybeUninit::<T>::uninit();
        unsafe {
            std::ptr::copy_nonoverlapping(read_bytes.as_ptr(), out.as_mut_ptr() as *mut u8, std::mem::size_of::<T>());
            return Ok(out.assume_init());
        }
    }
}
pub fn write<T: Copy + 'static>(target_process: &a_process, target_address: usize, value: T) -> Result<usize, MemError> {
    let bytes = unsafe {
        std::slice::from_raw_parts(&value as *const T as *const u8, std::mem::size_of::<T>())
    };
    write_bytes(target_process, target_address, bytes)
}
// these are kept for backwards compatability and for those who just want to use this for simplicity

pub fn read_f64(target_process: &a_process, target_address: usize) -> Result<f64, MemError> // wrapper
{
    return read::<f64>(target_process, target_address); // use read with the type and return the read or if it fails then 0.0 for float
}
pub fn read_f32(target_process: &a_process, target_address: usize) -> Result<f32, MemError> // wrapper
{
    return read::<f32>(target_process, target_address); // use read with the type and return the read or if it fails then 0.0 for float
}
pub fn read_u64(target_process: &a_process, target_address: usize) -> Result<u64, MemError> // wrapper
{
    return read::<u64>(target_process, target_address) // use read with the type and return the read or if it fails then 0
}
pub fn read_u32(target_process: &a_process, target_address: usize) -> Result<u32, MemError> // wrapper
{
    return read::<u32>(target_process, target_address) // use read with the type and return the read or if it fails then 0
}
pub fn read_u16(target_process: &a_process, target_address: usize) -> Result<u16, MemError> // wrapper
{
    return read::<u16>(target_process, target_address) // use read with the type and return the read or if it fails then 0
}
pub fn read_u8(target_process: &a_process, target_address: usize) -> Result<u8, MemError> // wrapper
{
    return read::<u8>(target_process, target_address) // use read with the type and return the read or if it fails then 0
}
pub fn read_i64(target_process: &a_process, target_address: usize) -> Result<i64, MemError> // wrapper
{
    return read::<i64>(target_process, target_address) // use read with the type and return the read or if it fails then 0
}
pub fn read_i32(target_process: &a_process, target_address: usize) -> Result<i32, MemError> // wrapper
{
    return read::<i32>(target_process, target_address); // use read with the type and return the read or if it fails then 0
}
pub fn read_i16(target_process: &a_process, target_address: usize) -> Result<i16, MemError> // wrapper
{
    return read::<i16>(target_process, target_address); // use read with the type and return the read or if it fails then 0
}
pub fn read_i8(target_process: &a_process, target_address: usize) -> Result<i8, MemError> // wrapper
{
    return read::<i8>(target_process, target_address); // use read with the type and return the read or if it fails then 0
}
pub fn read_usize(target_process: &a_process, target_address: usize) -> Result<usize, MemError> // wrapper
{
    return read::<usize>(target_process, target_address); // use read with the type and return the read or if it fails then 0
}
pub fn read_string(
    target_process: &a_process,
    target_address: usize,
    max_len: usize,
) -> Result<String, MemError> {
    if target_process.process_id == 0 || !process_alive(target_process.process_id) {
        return Err(MemError::InvalidProcess);
    }

    if target_address == 0 {
        return Err(MemError::InvalidAddress);
    }

    if max_len == 0 {
        return Ok(String::new());
    }

    let bytes = read_bytes(target_process, target_address, max_len)?;

    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(bytes.len());

    Ok(String::from_utf8_lossy(&bytes[..end]).to_string())
}
//
pub fn write_f64(target_process: &a_process, target_address: usize, value: f64) -> Result<usize, MemError> // wrapper
{
    return write::<f64>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0.0 for float
}
pub fn write_f32(target_process: &a_process, target_address: usize, value: f32) -> Result<usize, MemError> // wrapper
{
    return write::<f32>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0.0 for float
}
pub fn write_u64(target_process: &a_process, target_address: usize, value: u64) -> Result<usize, MemError> // wrapper
{
    return write::<u64>(target_process, target_address, value) // use read with the type and return the read or if it fails then 0
}
pub fn write_u32(target_process: &a_process, target_address: usize, value: u32) -> Result<usize, MemError> // wrapper
{
    return write::<u32>(target_process, target_address, value) // use read with the type and return the read or if it fails then 0
}
pub fn write_u16(target_process: &a_process, target_address: usize, value: u16) -> Result<usize, MemError> // wrapper
{
    return write::<u16>(target_process, target_address, value) // use read with the type and return the read or if it fails then 0
}
pub fn write_u8(target_process: &a_process, target_address: usize, value: u8) -> Result<usize, MemError> // wrapper
{
    return write::<u8>(target_process, target_address, value) // use read with the type and return the read or if it fails then 0
}
pub fn write_i64(target_process: &a_process, target_address: usize, value: i64) -> Result<usize, MemError> // wrapper
{
    return write::<i64>(target_process, target_address, value) // use read with the type and return the read or if it fails then 0
}
pub fn write_i32(target_process: &a_process, target_address: usize, value: i32) -> Result<usize, MemError> // wrapper
{
    return write::<i32>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0
}
pub fn write_i16(target_process: &a_process, target_address: usize, value: i16) -> Result<usize, MemError> // wrapper
{
    return write::<i16>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0
}
pub fn write_i8(target_process: &a_process, target_address: usize, value: i8) -> Result<usize, MemError> // wrapper
{
    return write::<i8>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0
}
pub fn write_usize(target_process: &a_process, target_address: usize, value: usize) -> Result<usize, MemError> // wrapper
{
    return write::<usize>(target_process, target_address, value); // use read with the type and return the read or if it fails then 0
}
pub fn write_bool(target_process: &a_process, target_address: usize, value: bool) -> Result<usize, MemError>  // use read withe the type and return if the read or if it fails then false
{
    return write::<u8>(target_process, target_address, if value == true { 1 } else { 0 } );
}

pub fn address_in_map(address: usize, maps: &Vec<a_module>) -> bool { // return if the address is in real memory space
    for map in maps { // for each module
        if address >= map.base && address < map.base + map.size { // check if the address is within the map
            return true; // return true
        }
    }
    return false; // return false for no theres no address in any map brah
}
pub fn resolve_pointer_chain(
    target_process: &a_process,
    base: usize,
    offsets: &[usize],
    read_last: bool,
) -> Result<usize, MemError> {
    if offsets.is_empty() {
        return Ok(base);
    }

    let mut current = base;

    for offset in offsets {
        let ptr = read_usize(target_process, current)?;
        current = ptr.wrapping_add(*offset);
    }
    if read_last {
        current = match read_usize(target_process, current) {
            Ok(c) => c,
            Err(e) => return Err(e)
        }
    }
    Ok(current)
}
