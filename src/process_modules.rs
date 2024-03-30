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
use std::{
    mem, slice, 
    collections::{BTreeMap, HashMap},
    sync::{Arc, OnceLock}
};
use tracing::{error, debug};
use widestring::*;
use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use parking_lot::{FairMutex, FairMutexGuard};
use crate::utils::TimeStamp;
use crate::event_trace::Image;


#[derive(Debug)]
pub struct ModuleInfo {
    pub file_name: String,
    pub time_data_stamp: u32
}

#[derive(Debug)]
pub struct ModuleInfoRunning {
    pub id: u32,
    pub module_info: Arc<ModuleInfo>,
    pub base_of_dll: u64,
    pub size_of_image: u32,
    pub entry_point: u64,
    pub start: TimeStamp,
    pub end: OnceLock<TimeStamp>,
}

static MODULES_MAP: Lazy<FairMutex<IndexMap<(String, u32), Arc<ModuleInfo>>>> = Lazy::new(|| {
    FairMutex::new(IndexMap::new())
});

static RUNNING_MODULES_MAP: Lazy<FairMutex<HashMap<u32, Arc<FairMutex<BTreeMap<u64, ModuleInfoRunning>>>>>> = Lazy::new(|| {
    FairMutex::new(HashMap::new())
});


pub fn process_modules_init(process_id: u32) {
    let process_module_mutex = match RUNNING_MODULES_MAP.lock().try_insert(process_id, Arc::new(FairMutex::new(BTreeMap::new()))) {
        Ok(ok) => ok.clone(),
        Err(ref err) => err.entry.get().clone()
    };

    let mut h_process_out = HANDLE::default();
    let oa = OBJECT_ATTRIBUTES{
        Length: mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        ..Default::default()};
    let status = unsafe{
        let client_id = CLIENT_ID{UniqueProcess: HANDLE(process_id as isize), UniqueThread: HANDLE::default()};
        NtOpenProcess(&mut h_process_out, GENERIC_ALL.0, &oa, Some(&client_id))
    };
    if status.is_err() {
        error!("Failed to NtOpenProcess: {}", status.0);
        return;
    }

    const MODULE_COUNT: usize = 1024;

    let mut module_array = Vec::<HMODULE>::with_capacity(MODULE_COUNT);
    let mut cbneeded = 0u32;
    loop {
        let cb = (mem::size_of::<HMODULE>() * module_array.capacity()) as u32;
        let r = unsafe{
            EnumProcessModulesEx(h_process_out, module_array.as_mut_ptr(), cb, &mut cbneeded, LIST_MODULES_ALL)
        };
        match r {
            Ok(_) => {
                if cbneeded > cb {
                    module_array = Vec::<HMODULE>::with_capacity(cbneeded as usize / mem::size_of::<HMODULE>());
                    continue;
                }
                unsafe{ module_array.set_len(cbneeded as usize / mem::size_of::<HMODULE>()) };
                break;
            },
            Err(e) => {
                unsafe{ ZwClose(h_process_out) };
                error!("Failed to EnumProcessModules: {}", e);
                return;
            }
        }
    }
    if !module_array.is_empty() {
        let mut vec = Vec::<u16>::with_capacity(1024);
        for i in 0..module_array.len() {
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
            let mut module_lock = MODULES_MAP.lock();
            let (id, module_info_arc) = if let Some(some) = module_lock.get_full(&(file_name.clone(), 0)) {
                (some.0, some.2.clone())
            } else {
                let module_info_arc = Arc::new(ModuleInfo{
                    file_name: file_name.clone(),
                    time_data_stamp: 0
                });
                let entry =  module_lock.insert_full((file_name.clone(), 0), module_info_arc.clone());
                (entry.0, entry.1.unwrap())
            };
            drop(module_lock);

            let mut process_module_lock = process_module_mutex.lock();
            let module_info_running = ModuleInfoRunning {
                id: id as u32,
                module_info: module_info_arc.clone(),
                base_of_dll: module_info.lpBaseOfDll as u64,
                size_of_image: module_info.SizeOfImage,
                entry_point: module_info.EntryPoint as u64,
                start: TimeStamp(0),
                end: OnceLock::new()
            };
            let _ = process_module_lock.try_insert(module_info.lpBaseOfDll as u64, module_info_running);
        }
    }
    unsafe{ ZwClose(h_process_out) };
}

pub fn process_modules_load(image_info: &Image) {
    // \\Device\\HarddiskVolume3\\Windows\\System32\\sechost.dll
    let file_name = if image_info.file_name.starts_with("\\") {
        String::new()
    } else {
        String::new()
    };
    let process_id = image_info.process_id;
    let mut module_lock = MODULES_MAP.lock();
    let (id, module_info_arc) = if let Some(some) = module_lock.get_full(&(file_name.clone(), image_info.time_date_stamp)) {
        (some.0, some.2.clone())
    } else {
        let module_info_arc = Arc::new(ModuleInfo{
            file_name: file_name.clone(),
            time_data_stamp: 0
        });
        let entry =  module_lock.insert_full((file_name.clone(), 0), module_info_arc.clone());
        (entry.0, entry.1.unwrap())
    };
    drop(module_lock);

    let process_module_mutex = match RUNNING_MODULES_MAP.lock().try_insert(process_id, Arc::new(FairMutex::new(BTreeMap::new()))) {
        Ok(ok) => ok.clone(),
        Err(ref err) => err.entry.get().clone()
    };
    let mut process_module_lock = process_module_mutex.lock();
    let module_info_running = ModuleInfoRunning {
        id: id as u32,
        module_info: module_info_arc.clone(),
        base_of_dll: image_info.image_base,
        size_of_image: image_info.image_size,
        entry_point: image_info.default_base,
        start: TimeStamp(0),
        end: OnceLock::new()
    };
    let _ = process_module_lock.try_insert(image_info.image_base, module_info_running);
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use super::RUNNING_MODULES_MAP;
    #[test]
    fn store_process_modules() {
        let current_id = unsafe{ GetCurrentProcessId() };
        let r = super::process_modules_init(current_id);
        println!("{:#?}", RUNNING_MODULES_MAP);
    }
}