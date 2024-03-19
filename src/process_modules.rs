use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES, System::SystemServices::{NtOpenProcess, ZwClose}
    }, 
    Win32::{
        Foundation::{GetLastError, GENERIC_ALL, HANDLE, HMODULE},
        System::{
            ProcessStatus::{EnumProcessModulesEx, GetModuleFileNameExW, GetModuleInformation, MODULEINFO, LIST_MODULES_ALL},
            WindowsProgramming::CLIENT_ID
        }
    }
};
use std::{mem, slice};
use tracing::debug;
use widestring::*;
use anyhow::{Result, anyhow};


#[derive(Debug)]
pub struct ModuleInfo {
    pub base_of_dll: u64,
    pub size_of_image: u32,
    pub entry_point: u64,
    pub file_name: String
}



pub fn store_process_modules(process_id: u32) -> Result<Vec<ModuleInfo>> {
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
        EnumProcessModulesEx(h_process_out, module_array.as_mut_ptr(), mem::size_of_val(&module_array) as u32, &mut cbneeded, LIST_MODULES_ALL)
    };
    if let Err(e) = r {
        unsafe{ ZwClose(h_process_out) };
        return Err(anyhow!("Failed to EnumProcessModules: {}", e));
    }
    let module_count = cbneeded/mem::size_of::<HMODULE>() as u32;
    let mut module_info_vec = Vec::<ModuleInfo>::with_capacity(module_count as usize);
    let mut vec = Vec::<u16>::with_capacity(1024);
    for i in 0..module_count {
        let status = unsafe{
            let slice = slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.capacity());
            GetModuleFileNameExW(h_process_out, module_array[i as usize], slice)
        };
        let file_name = if 0 == status {
            debug!("Failed to GetModuleFileNameExW: {}", unsafe{ GetLastError().unwrap_err() });
            String::new()
        } else {
            unsafe{
                U16CStr::from_ptr(vec.as_mut_ptr(), status as usize).unwrap_or_default()
            }.to_string().unwrap_or_else(|e| e. to_string())
        };

        let mut module_info = MODULEINFO::default();
        let r = unsafe{
            GetModuleInformation(h_process_out, module_array[i as usize], &mut module_info, mem::size_of::<MODULEINFO>() as u32)
        };
        if let Err(e) = r {
            debug!("Failed to GetModuleInformation: {}", e);
        }
        module_info_vec.push(ModuleInfo {
            base_of_dll: module_info.lpBaseOfDll as u64,
            size_of_image: module_info.SizeOfImage,
            entry_point: module_info.EntryPoint as u64,
            file_name: file_name})
    }
    unsafe{ ZwClose(h_process_out) };
    Ok(module_info_vec)
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    #[test]
    fn store_process_modules() {
        let current_id = unsafe{ GetCurrentProcessId() };
        let r = super::store_process_modules(current_id);
        assert!(r.is_ok());
        println!("{:#?}", r.unwrap());
    }
}