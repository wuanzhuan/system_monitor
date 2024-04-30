use crate::event_trace::EventRecordDecoded;
use crate::event_trace::Image;
use crate::third_extend::strings::AsPcwstr;
use crate::utils::TimeStamp;
use ascii::AsciiChar;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use std::{
    collections::{BTreeMap, HashMap},
    mem, slice,
    sync::{Arc, OnceLock},
    time::Duration
};
use tracing::{debug, error, warn};
use widestring::*;
use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        System::SystemServices::{NtOpenProcess, ZwClose},
    },
    Win32::{
        Foundation::*,
        Storage::FileSystem::QueryDosDeviceW,
        System::{
            Diagnostics::Etw,
            ProcessStatus::{
                EnumProcessModulesEx, GetModuleFileNameExW, GetModuleInformation, LIST_MODULES_ALL,
                MODULEINFO,
            },
            WindowsProgramming::CLIENT_ID,
        },
    },
};

#[derive(Debug)]
pub struct ModuleInfo {
    pub file_name: String,
    pub time_data_stamp: u32,
}

#[derive(Debug)]
pub struct ModuleInfoRunning {
    pub id: u32,
    pub module_info: Arc<ModuleInfo>,
    pub base_of_dll: u64,
    pub size_of_image: u32,
    pub entry_point: u64,
    pub start: TimeStamp,
}

static MODULES_MAP: Lazy<FairMutex<IndexMap<(String, u32), Arc<ModuleInfo>>>> =
    Lazy::new(|| FairMutex::new(IndexMap::new()));

static RUNNING_MODULES_MAP: Lazy<
    FairMutex<HashMap<u32, Arc<FairMutex<BTreeMap<u64, ModuleInfoRunning>>>>>,
> = Lazy::new(|| FairMutex::new(HashMap::new()));

static DRIVE_LETTER_MAP: OnceLock<HashMap<String, AsciiChar>> = OnceLock::new();

pub fn init() {
    drive_letter_map_init();
}

pub fn get_module_offset(process_id: u32, address: u64) -> Option<(/*module_id*/u32, /*offset*/u32)> {
    use std::ops::Bound;

    // is in kernel space
    if is_kernel_space(address) {
        if is_kernel_session_space(address) {
            // todo:
            None
        } else {
            // todo:
            None
        }
    } else {
        if let Some(process_module_mutex) = RUNNING_MODULES_MAP.lock().get(&process_id).cloned() {
            let process_module_lock = process_module_mutex.lock();
            let cursor = process_module_lock.upper_bound(Bound::Included(&address));
            if let Some(module_info_running) = cursor.value() {
                if address >= module_info_running.base_of_dll + module_info_running.size_of_image as u64 {
                    warn!("Cross the border address: {address:#x} the module start: {:#x} size: {:#x}", module_info_running.base_of_dll, module_info_running.size_of_image);
                    None
                } else {
                    Some((module_info_running.id,( address - module_info_running.base_of_dll) as u32))
                }
            } else {
                warn!("{address:#x} is not find in process_id: {process_id}");
                None
            }
        } else {
            warn!("Don't find process_id: {process_id} in RUNNING_MODULES_MAP");
            None
        }
    }
}

pub fn get_module_info_by_id(id: u32) -> Option<Arc<ModuleInfo>> {
    let lock = MODULES_MAP.lock();
    if let Some(entry) = lock.get_index(id as usize) {
        let module_info = entry.1.clone();
        drop(lock);
        Some(module_info)
    } else {
        None
    }
}

fn drive_letter_map_init() {
    let mut map = HashMap::<String, AsciiChar>::new();
    let mut file_name_ret = Vec::<u16>::with_capacity(260);
    unsafe {
        file_name_ret.set_len(file_name_ret.capacity());
    }
    for letter in 'c'..'z' {
        let file_name_raw = U16CString::from_str_truncate(format!("{letter}:"));
        match unsafe {
            QueryDosDeviceW(
                file_name_raw.as_pcwstr(),
                Some(file_name_ret.as_mut_slice()),
            )
        } {
            0 => {
                let err = unsafe { GetLastError().unwrap_err() };
                if err.code() != ERROR_FILE_NOT_FOUND.to_hresult() {
                    error!("Failed to QueryDosDeviceW: {err}");
                }
            }
            num => {
                unsafe {
                    file_name_ret.set_len(num as usize);
                }
                match U16CStr::from_slice_truncate(file_name_ret.as_slice()) {
                    Ok(ok) => {
                        map.insert(ok.to_string().unwrap(), AsciiChar::new(letter));
                    }
                    Err(err) => {
                        error!("Failed to from_ptr_truncate: {err}");
                    }
                }
            }
        };
    }
    let _ = DRIVE_LETTER_MAP.set(map);
}

pub fn handle_event_for_module(event_record: &mut EventRecordDecoded, is_selected: bool) {
    if is_selected {
        process_add(event_record.process_id);
    }
    match event_record.provider_id.0 {
        Etw::ProcessGuid => {
            if event_record.opcode_name == "End" {
                process_delete(event_record.process_id);
            }
        }
        Etw::ImageLoadGuid => {
            match event_record.opcode_name.as_str() {
                "Load" => {
                    let image = Image::from_event_record_decoded_with_mut(event_record, |disk_name| {
                        DRIVE_LETTER_MAP.get().unwrap().get(disk_name).map(|some| some.clone())
                    });
                    process_modules_load(&image, event_record.timestamp);
                }
                "UnLoad" => {
                    let image = Image::from_event_record_decoded_with_mut(event_record, |disk_name| {
                        DRIVE_LETTER_MAP.get().unwrap().get(disk_name).map(|some| some.clone())
                    });
                    process_modules_unload(&image);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn process_add(process_id: u32) {
    let mut running_module_lock = RUNNING_MODULES_MAP.lock();
    let process_module_mutex = match running_module_lock
        .try_insert(process_id, Arc::new(FairMutex::new(BTreeMap::new())))
    {
        Err(_) => {
            return;
        }
        Ok(ok) => ok.clone(),
    };
    drop(running_module_lock);

    smol::spawn(async move {
        let mut h_process_out = HANDLE::default();
        let oa = OBJECT_ATTRIBUTES {
            Length: mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        let status = unsafe {
            let client_id = CLIENT_ID {
                UniqueProcess: HANDLE(process_id as isize),
                UniqueThread: HANDLE::default(),
            };
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
            let r = unsafe {
                EnumProcessModulesEx(
                    h_process_out,
                    module_array.as_mut_ptr(),
                    cb,
                    &mut cbneeded,
                    LIST_MODULES_ALL,
                )
            };
            match r {
                Ok(_) => {
                    if cbneeded > cb {
                        module_array = Vec::<HMODULE>::with_capacity(
                            cbneeded as usize / mem::size_of::<HMODULE>(),
                        );
                        continue;
                    }
                    unsafe { module_array.set_len(cbneeded as usize / mem::size_of::<HMODULE>()) };
                    break;
                }
                Err(e) => {
                    unsafe { ZwClose(h_process_out) };
                    error!("Failed to EnumProcessModules: {}", e);
                    return;
                }
            }
        }
        if !module_array.is_empty() {
            let mut vec = Vec::<u16>::with_capacity(1024);
            for i in 0..module_array.len() {
                let status = unsafe {
                    let slice = slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.capacity());
                    GetModuleFileNameExW(h_process_out, module_array[i as usize], slice)
                };
                let file_name = if 0 == status {
                    debug!("Failed to GetModuleFileNameExW: {}", unsafe {
                        GetLastError().unwrap_err()
                    });
                    String::new()
                } else {
                    unsafe {
                        U16CStr::from_ptr(vec.as_mut_ptr(), status as usize).unwrap_or_default()
                    }
                    .to_string()
                    .unwrap_or_else(|e| e.to_string())
                };

                let mut module_info = MODULEINFO::default();
                let r = unsafe {
                    GetModuleInformation(
                        h_process_out,
                        module_array[i as usize],
                        &mut module_info,
                        mem::size_of::<MODULEINFO>() as u32,
                    )
                };
                if let Err(e) = r {
                    debug!("Failed to GetModuleInformation: {}", e);
                }
                let mut module_lock = MODULES_MAP.lock();
                let (id, module_info_arc) =
                    if let Some(some) = module_lock.get_full(&(file_name.clone(), 0)) {
                        (some.0, some.2.clone())
                    } else {
                        let module_info_arc = Arc::new(ModuleInfo {
                            file_name: file_name.clone(),
                            time_data_stamp: 0,
                        });
                        let entry = module_lock
                            .insert_full((file_name.clone(), 0), module_info_arc.clone());
                        (entry.0, module_info_arc)
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
                };
                let _ = process_module_lock
                    .try_insert(module_info.lpBaseOfDll as u64, module_info_running);
            }
        }
        unsafe { ZwClose(h_process_out) };
    })
    .detach();
}

fn process_delete(process_id: u32) {
    smol::spawn(async move {
        let period = Duration::from_secs(10);
        smol::Timer::after(period).await;
        let mut running_modules_lock = RUNNING_MODULES_MAP.lock();
        let _ = running_modules_lock.remove(&process_id);
    }).detach();
}

fn process_modules_load(image: &Image, timestamp: TimeStamp) {
    let mut module_lock = MODULES_MAP.lock();
    let (id, module_info_arc) = if let Some(some) =
        module_lock.get_full(&(image.file_name.clone(), image.time_date_stamp))
    {
        (some.0, some.2.clone())
    } else {
        let module_info_arc = Arc::new(ModuleInfo {
            file_name: image.file_name.clone(),
            time_data_stamp: 0,
        });
        let entry = module_lock.insert_full((image.file_name.clone(), image.time_date_stamp), module_info_arc.clone());
        (entry.0, module_info_arc)
    };
    drop(module_lock);

    let running_module_lock = RUNNING_MODULES_MAP.lock();
    let process_module_mutex = if let Some(process_module_mutex) = running_module_lock.get(&image.process_id){
        process_module_mutex
    } else {
        return;
    };
    let mut process_module_lock = process_module_mutex.lock();
    let module_info_running = ModuleInfoRunning {
        id: id as u32,
        module_info: module_info_arc.clone(),
        base_of_dll: image.image_base,
        size_of_image: image.image_size,
        entry_point: image.default_base,
        start: timestamp,
    };
    let _ = process_module_lock.try_insert(image.image_base, module_info_running);
}

fn process_modules_unload(image: &Image) {
    let running_module_lock = RUNNING_MODULES_MAP.lock();
    let process_module = if let Some(process_module_mutex) = running_module_lock.get(&image.process_id){
        process_module_mutex.clone()
    } else {
        return;
    };
    let image_base = image.image_base;
    smol::spawn(async move {
        let period = Duration::from_secs(5);
        smol::Timer::after(period).await;
        let mut process_module_lock = process_module.lock();
        let _ = process_module_lock.remove(&image_base);
    }).detach();
}

fn is_kernel_space(address: u64) -> bool {
    // fixme: 32 or 64
    if (address >> 48) == 0xffff {
        true
    } else {
        false
    }
}

fn is_kernel_session_space(address: u64) -> bool {
    // todo:
    false
}

#[cfg(test)]
mod tests {
    use super::{DRIVE_LETTER_MAP, RUNNING_MODULES_MAP};
    use windows::Win32::System::Threading::GetCurrentProcessId;

    #[test]
    fn store_process_modules() {
        let current_id = unsafe { GetCurrentProcessId() };
        let _ = super::process_add(current_id);
        println!("{:#?}", RUNNING_MODULES_MAP);
    }

    #[test]
    fn drive_letter_map_init() {
        super::drive_letter_map_init();
        println!("{:#?}", DRIVE_LETTER_MAP);
    }
}
