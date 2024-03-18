use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES, System::SystemServices::NtOpenProcess
    }, 
    Win32::{
        Foundation::{GetLastError, GENERIC_ALL, HANDLE, HMODULE, CloseHandle},
        System::{
            ProcessStatus::{EnumProcessModules, GetModuleFileNameExW, GetModuleInformation, MODULEINFO},
            WindowsProgramming::CLIENT_ID
        }
    }
};
use std::{mem::{self, size_of}, slice};
use tracing::error;
use widestring::*;
use anyhow::{Result, anyhow};



pub fn store_process_modules(process_id: u32) -> Result<()> {
    let mut h_process_out = HANDLE::default();
    let oa = OBJECT_ATTRIBUTES{
        Length: mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        ..Default::default()};
    let status = unsafe{
        let client_id = CLIENT_ID{UniqueProcess: HANDLE(process_id as isize), UniqueThread: HANDLE::default()};
        NtOpenProcess(&mut h_process_out, GENERIC_ALL.0, &oa, Some(&client_id))
    };
    if status.is_err() {
        return Err(anyhow!("Failed to NtOpenProcess: {}", status.0));
    }

    const MODULE_COUNT: usize = 1024;
    let mut module_array = [HMODULE::default(); MODULE_COUNT];
    let mut cbneeded = 0u32;
    let r = unsafe{
        EnumProcessModules(h_process_out, module_array.as_mut_ptr(), mem::size_of_val(&module_array) as u32, &mut cbneeded)
    };
    if let Err(e) = r {
        return Err(anyhow!("Failed to EnumProcessModules: {}", e));
    }
    let module_count = cbneeded/mem::size_of::<HMODULE>() as u32;

    let mut vec = Vec::<u16>::with_capacity(1024);
    let mut module_info_vec = Vec::<MODULEINFO>::with_capacity(module_count as usize);
    for i in 0..module_count {
        let status = unsafe{
            let slice = slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.capacity());
            GetModuleFileNameExW(h_process_out, module_array[i as usize], slice)
        };
        if 0 == status {
            let e = unsafe{ GetLastError().unwrap_err() };
            error!("Failed to GetModuleFileNameExW: {}", e);
        } else {
            let file_name = unsafe{
                U16CStr::from_ptr(vec.as_mut_ptr(), status as usize).unwrap_or_default()
            };
            println!("file name: {:?}", file_name);
        }

        let mut module_info = MODULEINFO::default();
        let r = unsafe{
            GetModuleInformation(h_process_out, module_array[i as usize], &mut module_info, mem::size_of::<MODULEINFO>() as u32)
        };
        if let Err(e) = r {
            error!("Failed to GetModuleInformation: {}", e);
        } else {
            println!("\t{:?}", module_info);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    #[test]
    fn store_process_modules() {
        let current_id = unsafe{ GetCurrentProcessId() };
        let r = super::store_process_modules(current_id);
        assert!(r.is_ok());
    }
}