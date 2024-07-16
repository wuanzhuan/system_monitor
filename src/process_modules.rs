use crate::{
    event_trace::{EventRecordDecoded, Image, Process, StackAddress},
    pdb::get_location_info as get_location_info_from_pdb,
    third_extend::strings::{AsPcwstr, StringEx},
    utils::TimeStamp,
};
use anyhow::{anyhow, Result};
use ascii::AsciiChar;
use indexmap::IndexMap;
use linked_hash_map::LinkedHashMap;
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use std::{
    cell::SyncUnsafeCell,
    collections::{BTreeMap, HashMap, VecDeque},
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem,
    ops::{Bound, Deref},
    path::Path,
    ptr, slice,
    sync::{Arc, OnceLock},
};
use tracing::{error, info, warn};
use widestring::*;
use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        System::{SystemServices::ZwClose, Threading::NtOpenProcess},
    },
    Win32::{
        Foundation::*,
        Storage::FileSystem::QueryDosDeviceW,
        System::{
            Diagnostics::{Debug::*, Etw},
            ProcessStatus::*,
            SystemServices::*,
            WindowsProgramming::CLIENT_ID,
        },
    },
};

static MODULES_MAP: Lazy<FairMutex<IndexMap<(String, u32), Arc<ModuleInfo>>>> =
    Lazy::new(|| FairMutex::new(IndexMap::new()));
static DRIVE_LETTER_MAP: OnceLock<HashMap<String, AsciiChar>> = OnceLock::new();

pub fn init() {
    drive_letter_map_init();
}

pub struct RunningModules {
    running_processes_modules: RunningProcessesModules,
    running_kernel_modules: RunningKernelModules,
}

impl RunningModules {
    pub fn new(max_count: usize, num_seconds: i64) -> Self {
        let mut running_processes_modules = RunningProcessesModules::new(max_count, num_seconds);
        running_processes_modules.processes_enum(&vec![]);
        let mut running_kernel_modules = RunningKernelModules::new();
        Self::enum_drivers(&mut running_kernel_modules);
        Self {
            running_processes_modules,
            running_kernel_modules,
        }
    }

    pub fn convert_to_module_offset(
        &self,
        process_id: u32,
        timestamp: TimeStamp,
        stacks: &mut [(String, StackAddress)],
    ) {
        let process_info_arc_option = self.running_processes_modules.get_or_insert(
            process_id,
            timestamp,
            "convert_to_module_offset",
        );

        let kernel_module_lock = self.running_kernel_modules.0.lock();
        for item in stacks.iter_mut() {
            if item.1.raw == 0 {
                continue;
            }
            // is in kernel space
            let address = item.1.raw;
            if is_kernel_space(address) {
                if is_kernel_session_space(address) {
                    // todo:
                } else {
                    let cursor = kernel_module_lock.upper_bound(Bound::Included(&address));
                    if let Some((_, module_info_running)) = cursor.peek_prev() {
                        if address
                            >= module_info_running.base_of_dll
                                + module_info_running.size_of_image as u64
                        {
                            warn!("Cross the border address: {address:#x} in the kernel. the module start: {:#x} size: {:#x} {}", 
                                module_info_running.base_of_dll, module_info_running.size_of_image, module_info_running.module_info.file_name);
                        } else {
                            item.1.relative = Some((
                                module_info_running.id,
                                (address - module_info_running.base_of_dll) as u32,
                            ));
                        }
                    } else {
                        warn!("{address:#x} is not find in kernel space");
                    }
                }
            } else {
                if let Some(ref process_info_arc) = process_info_arc_option {
                    let process_info = unsafe { &mut *process_info_arc.get() };
                    if let Some(ref e) = process_info.error {
                        if let ProcessError::NoModules(_) = e {
                            continue;
                        }
                    }
                    let cursor = process_info
                        .modules_map
                        .upper_bound(Bound::Included(&address));
                    if let Some((_, module_info_running)) = cursor.peek_prev() {
                        if address
                            >= module_info_running.base_of_dll
                                + module_info_running.size_of_image as u64
                        {
                            if process_info.error.is_none() {
                                //warn!("Cross the border address: {address:#x} in the [{process_id}]. the module start: {:#x} size: {:#x} {}",
                                //    module_info_running.base_of_dll, module_info_running.size_of_image, module_info_running.module_info.file_name);
                            }
                        } else {
                            item.1.relative = Some((
                                module_info_running.id,
                                (address - module_info_running.base_of_dll) as u32,
                            ));
                        }
                    } else {
                        if process_info.error.is_none() {
                            //warn!("{address:#x} is not find in process_id: {process_id}");
                        }
                    }
                }
            }
        }
    }

    pub fn get_process_path_by_id(&self, process_id: u32, timestamp: TimeStamp) -> String {
        if process_id == 0 {
            return String::from("System Idle");
        }
        if process_id == 4 || process_id as i32 == -1 {
            return String::from("System");
        }
        let process_info_arc = self
            .running_processes_modules
            .get_or_insert(process_id, timestamp, "get_process_path_by_id")
            .expect("Not special process should has process info");
        let process_info = unsafe { &*process_info_arc.get() };
        process_info.path.clone()
    }

    pub fn handle_event_for_module(&self, event_record: &mut EventRecordDecoded) {
        match event_record.provider_id.0 {
            Etw::ProcessGuid => match event_record.opcode_name.as_str() {
                "Start" => match Process::get_process_id_from_event_record_decoded(event_record) {
                    Ok(process_id) => self.process_start(process_id, event_record.timestamp),
                    Err(e) => error!(
                        "Failed to get the starting process id by process: {}, {e}",
                        event_record.process_id
                    ),
                },
                "End" => match Process::get_process_id_from_event_record_decoded(event_record) {
                    Ok(process_id) => self.process_end(process_id, event_record.timestamp),
                    Err(e) => error!(
                        "Failed to get the ending process id by process: {}, {e}",
                        event_record.process_id
                    ),
                },
                _ => {}
            },
            Etw::ImageLoadGuid => match event_record.opcode_name.as_str() {
                "Load" => {
                    let image =
                        Image::from_event_record_decoded_with_mut(event_record, |disk_name| {
                            DRIVE_LETTER_MAP
                                .get()
                                .unwrap()
                                .get(disk_name)
                                .map(|some| some.clone())
                        });
                    self.process_modules_load(&image, event_record.timestamp);
                }
                "UnLoad" => {
                    let image =
                        Image::from_event_record_decoded_with_mut(event_record, |disk_name| {
                            DRIVE_LETTER_MAP
                                .get()
                                .unwrap()
                                .get(disk_name)
                                .map(|some| some.clone())
                        });
                    self.process_modules_unload(&image, event_record.timestamp);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn process_start(&self, process_id: u32, start_time: TimeStamp) {
        let old_key = self.running_processes_modules.insert(
            process_id,
            Arc::new(SyncUnsafeCell::new(ProcessInfo {
                path: String::new(),
                create_time: start_time,
                start_time: Some(start_time),
                end_time: None,
                error: None,
                modules_map: BTreeMap::new(),
            })),
        );
        if old_key.is_some() {
            warn!("The new {process_id} of process id is coming but old has not removed");
        }
    }

    fn process_end(&self, process_id: u32, timestamp: TimeStamp) {
        // assume this event is last event for the process
        if self
            .running_processes_modules
            .remove(process_id, timestamp)
            .is_none()
        {
            warn!("No process: {process_id} when process end.")
        }
    }

    fn process_modules_load(&self, image: &Image, timestamp: TimeStamp) {
        let (id, module_info_arc) =
            module_map_insert(image.file_name.clone(), image.time_date_stamp);
        let module_info_running = ModuleInfoRunning {
            id: id as u32,
            module_info: module_info_arc.clone(),
            base_of_dll: image.image_base,
            size_of_image: image.image_size,
            entry_point: image.default_base,
            start: timestamp,
        };

        if is_kernel_space(image.image_base) {
            let _ = self
                .running_kernel_modules
                .lock()
                .try_insert(image.image_base, module_info_running);
        } else {
            if let Some(process_info_arc) = self.running_processes_modules.get_or_insert(
                image.process_id,
                timestamp,
                "process_modules_load",
            ) {
                let process_info = unsafe { &mut *process_info_arc.get() };
                // the main image must be the first image!
                if process_info.path.is_empty() && process_info.error.is_none() {
                    process_info.path = image.file_name.clone();
                }
                if let Ok(_new) = process_info
                    .modules_map
                    .try_insert(image.image_base, module_info_running)
                {
                    if let Some(ProcessError::NoModules(msg)) = process_info.error.take_if(|e| {
                        if let ProcessError::NoModules(_) = e {
                            true
                        } else {
                            false
                        }
                    }) {
                        process_info.error = Some(ProcessError::PartialModules(msg))
                    }
                }
            } else {
                error!(
                    "Can't get process: {} for non kernel module.",
                    image.process_id
                )
            }
        }
    }

    fn process_modules_unload(&self, image: &Image, timestamp: TimeStamp) {
        let process_id = image.process_id;
        let image_base = image.image_base;
        let file_name = image.file_name.clone();

        if is_kernel_space(image_base) {
            let _ = self.running_kernel_modules.lock().remove(&image_base);
        } else {
            if let Some(process_info_arc) = self.running_processes_modules.get_or_insert(
                process_id,
                timestamp,
                "process_modules_unload",
            ) {
                let process_info = unsafe { &mut *process_info_arc.get() };
                if process_info.modules_map.remove(&image_base).is_none() {
                    if process_info.error.is_none() {
                        warn!("No image: image_base: {image_base:#x} {file_name} when unloading in process: {process_id}");
                    }
                }
            } else {
                error!("Can't get process: {process_id} for non kernel module.");
            }
        }
    }

    fn enum_drivers(running_kernel_modules: &mut RunningKernelModules) {
        let mut driver_image_bases = vec![ptr::null_mut(); 100];
        let mut cb_needed = 0u32;
        loop {
            let cb = (driver_image_bases.len() * mem::size_of_val(&driver_image_bases[0])) as u32;
            match unsafe { EnumDeviceDrivers(driver_image_bases.as_mut_ptr(), cb, &mut cb_needed) }
            {
                Ok(_) => {
                    if cb_needed >= cb {
                        driver_image_bases = vec![ptr::null_mut(); driver_image_bases.len() * 2];
                        continue;
                    }
                    unsafe {
                        driver_image_bases
                            .set_len(cb_needed as usize / mem::size_of_val(&driver_image_bases[0]));
                    }
                    break;
                }
                Err(e) => {
                    error!("Failed to EnumProcesses: {e}");
                    return;
                }
            }
        }

        let system_root = std::env::var("SystemRoot").unwrap_or(String::from("C:\\Windows"));
        let mut vec = Vec::<u16>::with_capacity(MAX_PATH as usize);
        for image_base in driver_image_bases.iter() {
            let slice = unsafe { slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.capacity()) };
            let r = unsafe { GetDeviceDriverFileNameW(*image_base, slice) };
            let file_name = if 0 == r {
                warn!("Failed to GetDeviceDriverFileNameW: {:#?}", unsafe {
                    GetLastError()
                });
                continue;
            } else {
                unsafe { U16CStr::from_ptr(vec.as_mut_ptr(), r as usize).unwrap_or_default() }
                    .to_string()
                    .unwrap_or_else(|e| e.to_string())
            };
            const SYSTEM_ROOT: &str = "\\SystemRoot\\";
            const PREFIX: &str = "\\??\\";
            let file_name = if file_name.starts_with_case_insensitive(SYSTEM_ROOT) {
                format!("{system_root}\\{}", &file_name[SYSTEM_ROOT.len()..])
            } else if file_name.starts_with(PREFIX) {
                file_name[PREFIX.len()..].to_string()
            } else {
                file_name
            };
            let (image_size, time_date_stamp) =
                match get_image_info_from_file(Path::new(file_name.as_str())) {
                    Err(e) => {
                        info!("Failed to get_image_info_from_file: {file_name} {e}");
                        continue;
                    }
                    Ok(info) => info,
                };

            let (id, module_info_arc) = module_map_insert(file_name, time_date_stamp);
            let module_info_running = ModuleInfoRunning {
                id: id as u32,
                module_info: module_info_arc.clone(),
                base_of_dll: *image_base as u64,
                size_of_image: image_size,
                entry_point: 0,
                start: TimeStamp(0),
            };
            let _ = running_kernel_modules
                .lock()
                .try_insert(*image_base as u64, module_info_running);
        }
    }
}

struct RunningKernelModules(FairMutex<BTreeMap<u64, ModuleInfoRunning>>);

impl RunningKernelModules {
    fn new() -> Self {
        RunningKernelModules(FairMutex::new(BTreeMap::new()))
    }
}

impl Deref for RunningKernelModules {
    type Target = FairMutex<BTreeMap<u64, ModuleInfoRunning>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct RunningProcessesModules {
    map: SyncUnsafeCell<HashMap<u32, Arc<SyncUnsafeCell<ProcessInfo>>>>,
    ended_map: SyncUnsafeCell<LinkedHashMap<u32, VecDeque<Arc<SyncUnsafeCell<ProcessInfo>>>>>, // the process_id may be repeated. because of delay removing
    max_count: usize,
    num_seconds: i64,
}

impl RunningProcessesModules {
    fn new(max_count: usize, num_seconds: i64) -> Self {
        Self {
            map: SyncUnsafeCell::new(HashMap::new()),
            ended_map: SyncUnsafeCell::new(LinkedHashMap::new()),
            max_count,
            num_seconds,
        }
    }

    fn get(
        &self,
        process_id: u32,
        current_timestamp: TimeStamp,
    ) -> Result<Arc<SyncUnsafeCell<ProcessInfo>>, RunningProcessError> {
        if Self::is_special_process(process_id) {
            return Err(RunningProcessError::SpecialProcess(process_id));
        }
        let processes_map = unsafe { &mut *self.map.get() };
        if let Some(process_info_arc) = processes_map.get(&process_id) {
            return Ok(process_info_arc.clone());
        }
        let processes_ended_map = unsafe { &mut *self.ended_map.get() };
        if let Some(process_info_vec) = processes_ended_map.get(&process_id) {
            for process_info_arc in process_info_vec {
                let process_info = unsafe { &mut *process_info_arc.get() };
                // equal: include the self
                let start_time = process_info.start_time.unwrap_or(process_info.create_time);
                if current_timestamp.0 >= start_time.0
                    && (current_timestamp - process_info.end_time.unwrap()).num_seconds() < 10
                {
                    return Ok(process_info_arc.clone());
                }
            }
        }
        Err(RunningProcessError::NotFound(process_id))
    }

    fn get_or_insert(
        &self,
        process_id: u32,
        current_timestamp: TimeStamp,
        context: &str,
    ) -> Option<Arc<SyncUnsafeCell<ProcessInfo>>> {
        match self.get(process_id, current_timestamp) {
            Ok(process_info_arc) => {
                return Some(process_info_arc);
            }
            Err(e) => {
                if let RunningProcessError::SpecialProcess(_) = e {
                    return None;
                }
                warn!("{e} when {context}");
            }
        }
        let arc = Arc::new(SyncUnsafeCell::new(ProcessInfo {
            path: String::new(),
            create_time: current_timestamp,
            start_time: None,
            end_time: None,
            error: Some(ProcessError::NoModules(format!(
                "Insert after not discovering"
            ))),
            modules_map: BTreeMap::new(),
        }));
        let map = unsafe { &mut *self.map.get() };
        map.insert(process_id, arc.clone());
        Some(arc)
    }

    fn try_insert(
        &self,
        process_id: u32,
        process_info_arc: Arc<SyncUnsafeCell<ProcessInfo>>,
    ) -> Option<(/*is_new*/ bool, Arc<SyncUnsafeCell<ProcessInfo>>)> {
        if Self::is_special_process(process_id) {
            return None;
        }
        let processes_map = unsafe { &mut *self.map.get() };
        match processes_map.try_insert(process_id, process_info_arc) {
            Err(e) => Some((false, e.value)),
            Ok(proces_info) => Some((true, proces_info.clone())),
        }
    }

    fn insert(
        &self,
        process_id: u32,
        process_info_arc: Arc<SyncUnsafeCell<ProcessInfo>>,
    ) -> Option<Arc<SyncUnsafeCell<ProcessInfo>>> {
        if Self::is_special_process(process_id) {
            return None;
        }
        let processes_map = unsafe { &mut *self.map.get() };
        processes_map.insert(process_id, process_info_arc)
    }

    fn remove(
        &self,
        process_id: u32,
        current_timestamp: TimeStamp,
    ) -> Option<Arc<SyncUnsafeCell<ProcessInfo>>> {
        if Self::is_special_process(process_id) {
            return None;
        }
        let processes_map = unsafe { &mut *self.map.get() };
        if let Some(removed) = processes_map.remove(&process_id) {
            let process_info_removed = unsafe { &mut *removed.get() };
            debug_assert!(process_info_removed.end_time.is_none());
            process_info_removed.end_time = Some(current_timestamp);
            let ended_map = unsafe { &mut *self.ended_map.get() };
            if let Some(vec) = ended_map.get_mut(&process_id) {
                vec.push_back(removed.clone());
            } else {
                let mut vec = VecDeque::new();
                vec.push_back(removed.clone());
                ended_map.insert(process_id, vec);
            }
            self.clear_ended_map_front(current_timestamp);
            Some(removed)
        } else {
            None
        }
    }

    fn clear_ended_map_front(&self, current_timestamp: TimeStamp) {
        let ended_map = unsafe { &mut *self.ended_map.get() };
        for _index in 0..self.max_count {
            let mut is_pop = false;
            if let Some((_key, value)) = ended_map.front_mut() {
                loop {
                    if let Some(process_info_arc) = value.front() {
                        let process_info = unsafe { &mut *process_info_arc.get() };
                        let duration = current_timestamp - process_info.end_time.unwrap();
                        if duration.num_seconds() < self.num_seconds {
                            break;
                        }
                    } else {
                        is_pop = true;
                        break;
                    }
                    value.pop_front();
                }
            } else {
                break;
            }
            if is_pop {
                let _ = ended_map.pop_front();
            }
        }
    }

    #[inline]
    fn is_special_process(process_id: u32) -> bool {
        process_id == 0 || process_id == 4 || process_id as i32 == -1
    }

    // all process when filter_processes is empty
    fn processes_enum(&mut self, selected_process_ids: &Vec<u32>) {
        if !selected_process_ids.is_empty() {
            for id in selected_process_ids.iter() {
                self.process_init(*id);
            }
        } else {
            let mut process_ids = vec![0u32; 512];
            let mut cb_needed = 0u32;
            loop {
                let cb = (process_ids.len() * mem::size_of::<u32>()) as u32;
                match unsafe { EnumProcesses(process_ids.as_mut_ptr(), cb, &mut cb_needed) } {
                    Ok(_) => {
                        if cb_needed >= cb {
                            process_ids = vec![0u32; process_ids.len() * 2];
                            continue;
                        }
                        unsafe {
                            process_ids.set_len(cb_needed as usize / mem::size_of::<u32>());
                        }
                        break;
                    }
                    Err(e) => {
                        error!("Failed to EnumProcesses: {e}");
                        return;
                    }
                }
            }
            for id in process_ids.iter() {
                self.process_init(*id);
            }
        }
    }
    // only call before starting event trace
    fn process_init(&mut self, process_id: u32) {
        let process_info_arc = if let Some((is_new, process_info_arc)) = self.try_insert(
            process_id,
            Arc::new(SyncUnsafeCell::new(ProcessInfo {
                path: String::new(),
                create_time: TimeStamp::now(),
                start_time: None,
                end_time: None,
                error: None,
                modules_map: BTreeMap::new(),
            })),
        ) {
            if !is_new {
                return;
            }
            process_info_arc
        } else {
            return;
        };

        let mut h_process_out = HANDLE::default();
        let oa = OBJECT_ATTRIBUTES {
            Length: mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        let status = unsafe {
            let client_id = CLIENT_ID {
                UniqueProcess: HANDLE(process_id as *mut _),
                UniqueThread: HANDLE::default(),
            };
            NtOpenProcess(&mut h_process_out, GENERIC_ALL.0, &oa, Some(&client_id))
        };
        if status.is_err() {
            let err = ProcessError::NoModules(format!(
                "Failed to NtOpenProcess: {process_id}: {:#x} {}",
                status.0,
                status.to_hresult().message()
            ));
            if STATUS_ACCESS_DENIED != status {
                error!("{err}");
            }
            let process_info = unsafe { &mut *process_info_arc.get() };
            process_info.error = Some(err);
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
                    let _ = unsafe { ZwClose(h_process_out) };
                    let err = ProcessError::NoModules(format!(
                        "Failed to EnumProcessModules for {process_id}: {}",
                        e
                    ));
                    error!("{err}");
                    let process_info = unsafe { &mut *process_info_arc.get() };
                    process_info.error = Some(err);
                    return;
                }
            }
        }
        if !module_array.is_empty() {
            let mut vec = Vec::<u16>::with_capacity(1024);
            let process_info = unsafe { &mut *process_info_arc.get() };
            for i in 0..module_array.len() {
                let status = unsafe {
                    let slice = slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.capacity());
                    GetModuleFileNameExW(h_process_out, module_array[i as usize], slice)
                };
                let file_name = if 0 == status {
                    warn!("Failed to GetModuleFileNameExW: {:#?}", unsafe {
                        GetLastError()
                    });
                    String::new()
                } else {
                    unsafe {
                        U16CStr::from_ptr(vec.as_mut_ptr(), status as usize).unwrap_or_default()
                    }
                    .to_string()
                    .unwrap_or_else(|e| e.to_string())
                };
                if i == 0 {
                    process_info.path = file_name.clone();
                }

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
                    warn!("Failed to GetModuleInformation: {}", e);
                }
                let (_, time_date_stamp) =
                    match get_image_info_from_file(Path::new(file_name.as_str())) {
                        Ok(info) => info,
                        Err(e) => {
                            warn!("Failed to get_image_info_from_file: {e}");
                            (0, 0)
                        }
                    };
                let (id, module_info_arc) = module_map_insert(file_name.clone(), time_date_stamp);
                let module_info_running = ModuleInfoRunning {
                    id: id as u32,
                    module_info: module_info_arc.clone(),
                    base_of_dll: module_info.lpBaseOfDll as u64,
                    size_of_image: module_info.SizeOfImage,
                    entry_point: module_info.EntryPoint as u64,
                    start: TimeStamp(0),
                };
                let _ = process_info
                    .modules_map
                    .try_insert(module_info.lpBaseOfDll as u64, module_info_running);
            }
        }
        let _ = unsafe { ZwClose(h_process_out) };
    }
}

#[derive(Debug, thiserror::Error)]
enum RunningProcessError {
    #[error("SpecialProcess: {0}")]
    SpecialProcess(u32),
    #[error("NotFound: {0} in RunningProcesses")]
    NotFound(u32),
}

#[derive(Debug)]
pub struct ModuleInfo {
    pub file_name: String,
    pub time_data_stamp: u32,
}

impl ModuleInfo {
    pub fn get_module_name(&self) -> &str {
        get_file_name_from_path(self.file_name.as_str())
    }

    // offset: from module's image base
    pub fn get_location_info(
        &self,
        offset: u32,
    ) -> (/*function_offset*/ String, /*line_offset*/ String) {
        match get_location_info_from_pdb(
            Path::new(self.file_name.as_str()),
            self.time_data_stamp,
            offset,
        ) {
            Err(e) => {
                info!("{e:#}");
                (String::new(), String::new())
            }
            Ok(info) => info,
        }
    }
}

#[derive(Debug)]
pub struct ModuleInfoRunning {
    pub id: u32,
    pub module_info: Arc<ModuleInfo>,
    pub base_of_dll: u64,
    pub size_of_image: u32,
    #[allow(unused)]
    pub entry_point: u64,
    #[allow(unused)]
    pub start: TimeStamp,
}

#[derive(Debug)]
pub struct ProcessInfo {
    pub path: String,
    create_time: TimeStamp,        // the time when creating the ProcessInfo.
    start_time: Option<TimeStamp>, // the time when creating the process.
    end_time: Option<TimeStamp>,
    error: Option<ProcessError>,
    modules_map: BTreeMap<u64, ModuleInfoRunning>,
}

#[derive(thiserror::Error, Debug, PartialEq)]
enum ProcessError {
    #[error("No any module info: {0}")]
    NoModules(String),
    #[error("Has partial modules info: {0}")]
    PartialModules(String),
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

pub fn get_file_name_from_path(path: &str) -> &str {
    if let Some(offset) = path.rfind("\\") {
        path.get((offset + 1)..).unwrap_or("no_file_name")
    } else {
        path
    }
}

pub fn get_image_info_from_file(
    file_path: &Path,
) -> Result<(/*image_size*/ u32, /*time_data_stamp*/ u32)> {
    let mut file = match File::open(file_path) {
        Err(e) => {
            return Err(anyhow!("Failed to open file: {} {e}", file_path.display()));
        }
        Ok(file) => file,
    };
    let mut data = vec![0u8; mem::size_of::<IMAGE_DOS_HEADER>()];
    let nt_header_offset = match file.read(&mut data) {
        Err(e) => {
            return Err(anyhow!("Faile to read file: {} {e}", file_path.display()));
        }
        Ok(size) => {
            if size != mem::size_of::<IMAGE_DOS_HEADER>() {
                return Err(anyhow!(
                    "The return size: {size} is not equal mem::size_of::<IMAGE_DOS_HEADER>()"
                ));
            }
            let dos_header: &IMAGE_DOS_HEADER = unsafe { mem::transmute(data.as_ptr()) };
            dos_header.e_lfanew
        }
    };

    let mut data = vec![0u8; mem::size_of::<IMAGE_NT_HEADERS64>()];
    if let Err(e) = file.seek(SeekFrom::Start(nt_header_offset as u64)) {
        return Err(anyhow!("Failed to seek file: {} {e}", file_path.display()));
    }
    match file.read(&mut data) {
        Err(e) => {
            return Err(anyhow!("Faile to read file: {} {e}", file_path.display()));
        }
        Ok(size) => {
            if size != mem::size_of::<IMAGE_NT_HEADERS64>() {
                return Err(anyhow!(
                    "The return size: {size} is not equal mem::size_of::<IMAGE_NT_HEADERS64>()"
                ));
            }
            let nt_header: &IMAGE_NT_HEADERS64 = unsafe { mem::transmute(data.as_ptr()) };
            let time_data_stamp = nt_header.FileHeader.TimeDateStamp;
            let image_size = if nt_header.FileHeader.SizeOfOptionalHeader
                == mem::size_of::<IMAGE_NT_HEADERS64>() as u16
            {
                nt_header.OptionalHeader.SizeOfImage
            } else {
                let nt_header: &IMAGE_NT_HEADERS32 = unsafe { mem::transmute(data.as_ptr()) };
                nt_header.OptionalHeader.SizeOfImage
            };
            Ok((image_size, time_data_stamp))
        }
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
                let err = unsafe { GetLastError() };
                if err != ERROR_FILE_NOT_FOUND {
                    error!("Failed to QueryDosDeviceW: {err:#?}");
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

fn module_map_insert(
    file_name: String,
    time_date_stamp: u32,
) -> (/*id*/ usize, /*id*/ Arc<ModuleInfo>) {
    let mut module_lock = MODULES_MAP.lock();
    if let Some(some) = module_lock.get_full(&(file_name.clone(), time_date_stamp)) {
        (some.0, some.2.clone())
    } else {
        let module_info_arc = Arc::new(ModuleInfo {
            file_name: file_name.clone(),
            time_data_stamp: time_date_stamp,
        });
        let entry = module_lock.insert_full(
            (file_name.clone(), time_date_stamp),
            module_info_arc.clone(),
        );
        (entry.0, module_info_arc)
    }
}

fn is_kernel_space(address: u64) -> bool {
    // fixme: 32 or 64
    if (address >> 48) == 0xffff {
        true
    } else {
        false
    }
}

fn is_kernel_session_space(_address: u64) -> bool {
    // todo:
    false
}

#[cfg(test)]
mod tests {
    use super::{RunningKernelModules, RunningModules, RunningProcessesModules, DRIVE_LETTER_MAP};
    use crate::pdb::pdb_path_set;
    use std::path::Path;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    #[test]
    fn store_process_modules() {
        let current_id = unsafe { GetCurrentProcessId() };
        let mut running_processes_modules = RunningProcessesModules::new(5, 10);
        running_processes_modules.process_init(current_id);

        println!("{:#?}", running_processes_modules.0);
    }

    #[test]
    fn drive_letter_map_init() {
        super::drive_letter_map_init();
        println!("{:#?}", DRIVE_LETTER_MAP);
    }

    #[test]
    fn enum_processes() {
        let mut running_processes_modules = RunningProcessesModules::new(5, 10);
        running_processes_modules.processes_enum(&vec![]);
        println!("{:#?}", running_processes_modules.0)
    }

    #[test]
    fn enum_drivers() {
        let mut running_kernel_modules = RunningKernelModules::new();
        RunningModules::enum_drivers(&mut running_kernel_modules);
        println!("{:#?}", running_kernel_modules.0);
    }

    #[test]
    fn get_location_info() {
        let out_dir = env!("CARGO_MANIFEST_DIR");
        let pkg_name = env!("CARGO_PKG_NAME");
        let (_, time_date_stamp) = super::get_image_info_from_file(Path::new(
            format!("{out_dir}\\target\\debug\\{pkg_name}.exe").as_str(),
        ))
        .unwrap();
        pdb_path_set(format!("{out_dir}\\target\\debug").as_str());
        let module_info = super::ModuleInfo {
            file_name: format!("{out_dir}\\target\\debug\\{pkg_name}.exe"),
            time_data_stamp: time_date_stamp,
        };
        let r = module_info.get_location_info(0x2b6168);
        println!("{r:?}");
    }
}
