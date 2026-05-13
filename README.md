# linux memory

Fast and simple external memory reading library for Linux

## Features
- `process_vm_readv` based reading (will get stealthier in the future)
- Optional configurable random delay
- Clean API
- Easy process and module finding

## Example

```rust
use linux_memkit::*;

fn test() -> Result<(), MemError> {
    let cs2 = match find_process_from_name("cs2".to_string()) {
        Ok(c) => c,
        Err(e) => {
            println!("procerror");
            return Err(MemError::InvalidProcess);
        }
    };
    let client = match find_module_from_name("client".to_string(), &cs2, true) {
        Ok(c) => c,
        Err(e) => {
            println!("maperror");
            return Err(MemError::InvalidMap);
        }
    };
    let local_pawn = read_u64(&cs2, client.base + 0x206A9E0)?;
    let health = read_i32(&cs2, local_pawn as usize + 0x354)?;
    println!("CS2 -> {}\nClient -> {:016X}\nLocal Pawn -> {:016X}\nHealth -> {}", cs2.process_id, client.base, local_pawn, health);

    return Ok(());
}
//
// or if it bothers you that much to do Results and safety
//
fn find_process_from_name_wrapper(target_process_name: String) -> a_process {
    return match find_process_from_name(target_process_name) {
        Ok(p) => p,
        Err(_e) => a_process { process_id: 0, file_name: "".to_string(), cmd_line: vec![] }
    };
}
fn find_module_from_name_wrapper(target_module_name: String, target_process: &a_process, case_insensitive: bool) -> a_module {
    return match find_module_from_name(target_module_name, target_process, case_insensitive) {
        Ok(m) => m,
        Err(_e) => a_module { name: "".to_string(), base: 0, size: 0 }
    };
}
fn read_u64_wrapper(target_process: &a_process, target_address: usize) -> u64 {
    return match read_u64(target_process, target_address) {
        Ok(v) => v,
        Err(_e) => 0 // _ is not needed its just to silence the warning
    };
}
fn read_i32_wrapper(target_process: &a_process, target_address: usize) -> i32 {
    return match read_i32(target_process, target_address) {
        Ok(v) => v,
        Err(_e) => 0 // _ is not needed its just to silence the warning
    };
}
fn test_with_wrapper() {
    let cs2 = find_process_from_name_wrapper("cs2".to_string());
    /*
    if cs2.process_id == 0 {
        println!("failed to find process");
        return;
    }
    */
    let client = find_module_from_name_wrapper("client".to_string(), &cs2, true);
    /*
    if client.base <= 0 && client.size <= 0 {
        println!("failed to find module");
        return;
    }
    */
    let local_pawn = read_u64_wrapper(&cs2, client.base + 0x206A9E0);
    /*
    if local_pawn <= 0 {
        println!("failed to read local_pawn");
        return; 
    } 
    */
    let health = read_i32_wrapper(&cs2, local_pawn as usize + 0x354);
    println!("CS2 -> {}\nClient -> {:016X}\nLocal Pawn -> {:016X}\nHealth -> {}", cs2.process_id, client.base, local_pawn, health);

    return;
}