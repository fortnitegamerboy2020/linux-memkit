use std::{ffi::c_void, sync::atomic::{AtomicBool, AtomicUsize, Ordering}, thread::sleep, vec};

use libc::{iovec, process_vm_readv};
use procfs::{self, process::Process};
use rand::{RngExt}; 
static DELAYED_READS_ENABLED: AtomicBool = AtomicBool::new(false);
static DELAYED_READ_MIN: AtomicUsize = AtomicUsize::new(5);
static DELAYED_READ_MAX: AtomicUsize = AtomicUsize::new(20);

pub fn random(start: usize, end: usize) -> usize
{
    let mut range = rand::rng();
    return range.random_range(start..=end);
}

pub struct a_process
{
    pub process: Option<Process>,
    pub process_id: u32,
    pub file_name: String,
    pub cmd_line: Vec<String>
}

pub struct a_module {
    pub name: String,      // e.g. "client.dll"
    pub base: u64,         // base address
    pub size: u64,         // size of the module
}
pub fn find_processes() -> Vec<a_process>
{
    let mut processes : Vec<a_process> = vec![];
    for proc in procfs::process::all_processes().unwrap()
    {
        let Ok(process) = proc else { continue; };
        let Ok(exe_path) = process.exe() else { continue; };
        let Ok(cmdline) = process.cmdline() else { continue; };

        let file_name = exe_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let process_id: u32 = process.pid() as u32;
        processes.push(a_process { process: Some(process), process_id, file_name, cmd_line: cmdline });

    }
    return processes;
}
pub fn find_process_from_process_id(target_process_id: u32) -> a_process
{

    for current_process in find_processes()
    {
        if current_process.process_id == target_process_id
        {
            return current_process;
        }
    }
    return a_process { process: None, process_id: 0, file_name: String::from(""), cmd_line:  vec![]};
}
pub fn find_process_from_name(target_process_name: String) -> a_process
{
    for process in find_processes()
    {
        if process.file_name.contains(target_process_name.as_str())
        {
            return process;
        }
    }
    return a_process { process: None, process_id: 0, file_name: String::from(""), cmd_line: vec![] };
}
pub fn find_module_from_name(target_module_name: String, target_process: &a_process) -> a_module {
    let process = match &target_process.process {
        Some(p) => p,
        None => return a_module { 
            name: String::from(""), 
            base: 0x0, 
            size: 0x0 
        },
    };

    let Ok(maps) = process.maps() else {
        return a_module { 
            name: String::from(""), 
            base: 0x0, 
            size: 0x0 
        };
    };

    let lower_target = target_module_name.to_lowercase();

    for map in maps {
        let pathname = match &map.pathname {
            procfs::process::MMapPath::Path(path) => path,
            _ => continue,
        };

        let file_name_lower = pathname.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if file_name_lower.contains(&lower_target) || file_name_lower == lower_target {
            let (start_addr, end_addr) = map.address;
            let real_name = pathname.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown.so")
                .to_string();

            return a_module {
                name: real_name,
                base: start_addr,
                size: end_addr - start_addr,
            };
        }
    }

    // Not found
    a_module {
        name: String::from(""),
        base: 0x0,
        size: 0x0,
    }
}
pub fn enable_read_sleep()
{
    DELAYED_READS_ENABLED.store(true, Ordering::Relaxed);
}
pub fn disable_read_sleep()
{
    DELAYED_READS_ENABLED.store(false, Ordering::Relaxed)
}
pub fn set_read_min_sleep(delay: usize)
{
    DELAYED_READ_MIN.store(delay, Ordering::Relaxed)
}
pub fn set_read_max_sleep(delay: usize)
{
    DELAYED_READ_MAX.store(delay, Ordering::Relaxed)
}
pub fn set_delay_range(min_ms: usize, max_ms: usize) {
    DELAYED_READ_MIN.store(min_ms, Ordering::Relaxed);
    DELAYED_READ_MAX.store(max_ms, Ordering::Relaxed);
}
pub fn read_u64(target_process: &a_process, target_address: u64) -> u64
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed)
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    
    let size: usize = std::mem::size_of::<u64>();
    let mut buffer: u64 = 0x0;
    let local_iov = iovec {
        iov_base: &mut buffer as *mut u64 as *mut c_void,
        iov_len: size
    };
    let remote_iov = iovec {
        iov_base: target_address as *mut c_void,
        iov_len: size
    };
    let result_read: usize = unsafe { 
        process_vm_readv(target_process.process_id as i32, 
            &local_iov as *const iovec, 1, 
            &remote_iov as *const iovec, 1, 
            0) as usize 
    };
    if result_read == size
    {
        return buffer;
    }
    
    return 0x0;
}
pub fn read_u32(target_process: &a_process, target_address: u64) -> u32
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed)
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    
    let size: usize = std::mem::size_of::<u32>();
    let mut buffer: u32 = 0x0;
    let local_iov = iovec {
        iov_base: &mut buffer as *mut u32 as *mut c_void,
        iov_len: size
    };
    let remote_iov = iovec {
        iov_base: target_address as *mut c_void,
        iov_len: size
    };
    let result_read: usize = unsafe { 
        process_vm_readv(target_process.process_id as i32, 
            &local_iov as *const iovec, 1, 
            &remote_iov as *const iovec, 1, 
            0) as usize 
    };
    if result_read == size
    {
        return buffer;
    }
    
    return 0x0;
}
pub fn read_u16(target_process: &a_process, target_address: u64) -> u16
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed)
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    
    let size: usize = std::mem::size_of::<u16>();
    let mut buffer: u16 = 0x0;
    let local_iov = iovec {
        iov_base: &mut buffer as *mut u16 as *mut c_void,
        iov_len: size
    };
    let remote_iov = iovec {
        iov_base: target_address as *mut c_void,
        iov_len: size
    };
    let result_read: usize = unsafe { 
        process_vm_readv(target_process.process_id as i32, 
            &local_iov as *const iovec, 1, 
            &remote_iov as *const iovec, 1, 
            0) as usize 
    };
    if result_read == size
    {
        return buffer;
    }
    
    return 0x0;
}
pub fn read_u8(target_process: &a_process, target_address: u64) -> u8
{
    if DELAYED_READS_ENABLED.load(Ordering::Relaxed)
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    
    let size: usize = std::mem::size_of::<u8>();
    let mut buffer: u8 = 0x0;
    let local_iov = iovec {
        iov_base: &mut buffer as *mut u8 as *mut c_void,
        iov_len: size
    };
    let remote_iov = iovec {
        iov_base: target_address as *mut c_void,
        iov_len: size
    };
    let result_read: usize = unsafe { 
        process_vm_readv(target_process.process_id as i32, 
            &local_iov as *const iovec, 1, 
            &remote_iov as *const iovec, 1, 
            0) as usize 
    };


    if result_read == size
    {
        return buffer;
    }
    
    return 0x0;
}
pub fn read_usize(target_process: &a_process, target_address: u64) -> usize
{
	if DELAYED_READS_ENABLED.load(Ordering::Relaxed)
	{
		sleep(std::time::Duration::from_millis(random(DELAYED_READ_MIN.load(Ordering::Relaxed), DELAYED_READ_MAX.load(Ordering::Relaxed)) as u64));
	}
    
    let size: usize = std::mem::size_of::<usize>();
    let mut buffer: usize = 0x0;
    let local_iov = iovec {
        iov_base: &mut buffer as *mut usize as *mut c_void,
        iov_len: size
    };
    let remote_iov = iovec {
        iov_base: target_address as *mut c_void,
        iov_len: size
    };
    let result_read: usize = unsafe { 
        process_vm_readv(target_process.process_id as i32, 
            &local_iov as *const iovec, 1, 
            &remote_iov as *const iovec, 1, 
            0) as usize 
    };
    if result_read == size
    {
        return buffer;
    }
    
    return 0x0;
}

