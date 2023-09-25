use std::{
    fmt,
    ffi,
    mem,
    ptr,
    slice,
    thread,
    sync::{Arc, Mutex, mpsc::{self, RecvTimeoutError}},
    time::Duration
};

use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::System::Diagnostics::Etw::*,
    Win32::System::SystemInformation::*
};
use widestring::*;
use crate::third_extend::bytemuck::*;
use crate::third_extend::strings::*;
use tracing::{error, warn, info};
use lazy_static::lazy_static;
use chrono::*;


mod event_kernel;


const SESSION_NAME_SYSMON: &U16CStr  = u16cstr!("sysmonx");
const SESSION_NAME_NT: &U16CStr = u16cstr!("NT Kernel Logger");
const INVALID_PROCESSTRACE_HANDLE: u64 =  if cfg!(target_pointer_width = "64") { 0xffffffff_ffffffff } else{ 0x00000000_ffffffff };

// {ADA6BC38-93C9-00D1-7462-11D6841904AA}
const DUMMY_GUID: GUID = GUID{ data1: 0xADA6BC38, data2: 0x93C9, data3: 0x00D1, data4: [0x74, 0x62, 0x11, 0xD6, 0x84, 0x19, 0x04, 0xAA] };

#[repr(C)]
struct EtwPropertiesBuf(EVENT_TRACE_PROPERTIES, [u8]);
struct ConfigKernel{
    is_selected: bool,
    event_desc: &'static event_kernel::EventsDescribe
}

pub struct Controller{
    h_trace_session: CONTROLTRACE_HANDLE,
    h_trace_consumer: PROCESSTRACE_HANDLE,
    h_consumer_thread: Option<thread::JoinHandle<()>>,
    config: Vec::<ConfigKernel>,
}

pub type FnCompletion = fn(Result<()>);

pub struct EventRecordDecoded {
    task_name: String,
    opcode_name: String
}

struct EventRecord<'a>(&'a EVENT_RECORD);





lazy_static!{
    static ref CONTEXT: Arc::<Mutex<Controller>> = Arc::new(Mutex::new(Controller::new()));
}

impl Controller{
    fn new() -> Self {
        let mut cxt = Self{h_trace_session: CONTROLTRACE_HANDLE::default(), h_trace_consumer: PROCESSTRACE_HANDLE{Value: INVALID_PROCESSTRACE_HANDLE}, h_consumer_thread: None, config: Vec::<ConfigKernel>::new()};
        for item in event_kernel::EVENTS_DESC.iter() {
            cxt.config.push( ConfigKernel{is_selected: false, event_desc: item});
        };
        cxt
    }

    pub fn start(fn_completion: FnCompletion) -> Result<()>{
        let context_arc = CONTEXT.clone();
        let mut context_mg = context_arc.try_lock().map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?;
        unsafe {
            let mut h_trace = CONTROLTRACE_HANDLE::default();
            let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
            let session_name: &U16CStr = if is_win8_or_greater { SESSION_NAME_SYSMON } else { SESSION_NAME_NT};
            let mut properties_buf = make_properties(is_win8_or_greater, session_name);
    
            let mut error: Result<()>;
            loop {
                error = StartTraceW(&mut h_trace, session_name.as_pcwstr(), &mut properties_buf.0);
                match error {
                    Ok(_) => {
                        context_mg.h_trace_session =  h_trace;
                        break;
                    },
                    Err(e) => {
                        if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                            let error = ControlTraceW(CONTROLTRACE_HANDLE::default(), session_name.as_pcwstr(), &mut properties_buf.0, EVENT_TRACE_CONTROL_STOP);
                            if error.is_ok() {
                                warn!("The {session_name:#?} is already exist. and stop before restart");
                                continue;
                            }
                            error!("The {session_name:#?} is already exist. And failed to stop: {:#?}", error);
                            return error;
                        }
                        error!("Failed to StartTraceW: {:#?}", e);
                        return Err(e);
                    }
                }
            }

            context_mg.update_config()?;

            unsafe extern "system" fn callback(eventrecord: *mut EVENT_RECORD){
                let er = mem::transmute(eventrecord);
                info!("{}", EventRecord(er));
            }
    
            let mut trace_log = EVENT_TRACE_LOGFILEW{
                Context: &mut*context_mg as *mut Controller as *mut ffi::c_void,
                LoggerName:  PWSTR::from_raw(session_name.as_ptr() as *mut u16),
                Anonymous1: EVENT_TRACE_LOGFILEW_0{ ProcessTraceMode:  PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_REAL_TIME },
                Anonymous2: EVENT_TRACE_LOGFILEW_1{ EventRecordCallback: Some(callback)},
                ..Default::default()
            };
            let h_consumer = OpenTraceW(&mut trace_log);
            if INVALID_PROCESSTRACE_HANDLE == h_consumer.Value {
                return Err(Error::from_win32());
            }
            context_mg.h_trace_consumer = h_consumer;

            let (tx, rx) = mpsc::channel::<Error>();
            let h_thread = thread::spawn(move ||{
                let ft_now = GetSystemTimeAsFileTime();
                let r_pt = ProcessTrace(&[h_consumer], Some(&ft_now), None);
                if let Err(e) = r_pt.clone() {
                    error!("Failed to ProcessTrace: {}", e);
                    let r_send = tx.send(e);
                    if r_send.is_ok() {
                        return;
                    }
                }
                let context_arc = CONTEXT.clone();
                let mut context_mg = context_arc.lock().unwrap();
                context_mg.h_consumer_thread = None;
                fn_completion(r_pt);
            });
            let r_recv = rx.recv_timeout(Duration::from_millis(200));
            if let Err(e) = r_recv {
                if e == RecvTimeoutError::Timeout {
                    context_mg.h_consumer_thread = Some(h_thread);
                    return Ok(());
                }
                error!("{}", e);
                return Err(E_FAIL.into());
            }else {
                let e = r_recv.unwrap();
                error!("{}", e);
                return Err(e);
            }
        }
    }
    
    pub fn stop() -> Result<()>{
        let context_arc = CONTEXT.clone();
        let mut context_mg = context_arc.try_lock().map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?;
        unsafe{
            if 0 != context_mg.h_trace_session.Value {
                let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
                let session_name: &U16CStr = if is_win8_or_greater { SESSION_NAME_SYSMON } else { SESSION_NAME_NT };
                let mut properties_buf = make_properties(is_win8_or_greater, session_name);
                let error = ControlTraceW(context_mg.h_trace_session, session_name.as_pcwstr(), &mut properties_buf.0, EVENT_TRACE_CONTROL_STOP);
                context_mg.h_trace_session.Value = 0;
                if let Err(e) = error {
                    error!("failed to ControlTraceW {}", e);
                }
            }

            if INVALID_PROCESSTRACE_HANDLE != context_mg.h_trace_consumer.Value {
                let error = CloseTrace(context_mg.h_trace_consumer);
                context_mg.h_trace_consumer.Value = INVALID_PROCESSTRACE_HANDLE;
                if let Err(e) = error {
                    if ERROR_CTX_CLOSE_PENDING.to_hresult() != e.code() {
                        error!("failed to CloseTrace {}", e);
                    }
                }
            }
        }
        if context_mg.h_consumer_thread.is_some() {
            let h = context_mg.h_consumer_thread.take().unwrap();
            mem::drop(context_mg);
            let _ = h.join();
        }
        Ok(())
    }

    fn update_config(&self)-> Result<()> {
        
        #[derive(Default)]
        struct PerfInfoGroupMask{
            masks:[u32; 8]
        }
        let mut gm = PerfInfoGroupMask::default();
        gm.masks[0] = EVENT_TRACE_FLAG_PROCESS.0;
        for item in self.config.iter() {
            if !item.is_selected {continue}
            gm.masks[(item.event_desc.major.flag >> 32) as usize] |= item.event_desc.major.flag as u32;
        }
        unsafe{
            TraceSetInformation(self.h_trace_session, TraceSystemTraceEnableFlagsInfo, ptr::addr_of!(gm.masks) as *const ffi::c_void, std::mem::size_of_val(&gm.masks) as u32)?;
            let mut vec_event_id = Vec::<CLASSIC_EVENT_ID>::with_capacity(32);
            for item in self.config.iter(){
                if !item.is_selected {continue}
                for item_minor in item.event_desc.minors.iter(){
                    let mut id = CLASSIC_EVENT_ID::default();
                    id.EventGuid = item.event_desc.guid;
                    id.Type = item_minor.op_code as u8;
                    vec_event_id.push(id);
                }
            }
    
            TraceSetInformation(self.h_trace_session, TraceStackTracingInfo, vec_event_id.as_ptr() as *const ffi::c_void, vec_event_id.len() as u32)?;
        }
        Ok(())
    }
}

fn make_properties(is_win8_or_greater: bool, session_name: &U16CStr) -> Box<EtwPropertiesBuf>{
    let properties_buf_len = mem::size_of::<EVENT_TRACE_PROPERTIES>() + session_name.len() * 2 + 2;
    let properties_buf = vec![0u8; properties_buf_len].leak() as *mut[u8] as *mut EtwPropertiesBuf;
    unsafe{
        let mut properties_buf = Box::from_raw(properties_buf);
        let properties = &mut(*properties_buf).0;
        properties.EnableFlags = EVENT_TRACE_FLAG_PROCESS;
        properties.Wnode.BufferSize = properties_buf_len as u32;
        properties.Wnode.Guid = if is_win8_or_greater { DUMMY_GUID } else { SystemTraceControlGuid };
        properties.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        properties.Wnode.ClientContext = 1;
        properties.FlushTimer = 1;
        properties.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE | EVENT_TRACE_REAL_TIME_MODE | EVENT_TRACE_USE_LOCAL_SEQUENCE;
        properties.LoggerNameOffset = mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

        properties_buf
    }
}

impl<'a> fmt::Display for EventRecord<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = &self.0.EventHeader;
        let duration = Utc.ymd(1970, 1, 1) - Utc.ymd(1601, 1, 1);
        let dt_utc = Utc.timestamp_millis(header.TimeStamp / 10 / 1000 - duration.num_milliseconds());
        let dt_local: DateTime<Local> = dt_utc.into();

        if (header.Flags & EVENT_HEADER_FLAG_TRACE_MESSAGE as u16) != 0 {
            return write!(f, "wpp event. ProcessId: {:+} ThreadId: {:+} {}", header.ProcessId, header.ThreadId, dt_local);
        }
        const BUF_SIZE: usize = 4096;
        let mut buffer_size = BUF_SIZE as u32;
        let mut event_info: &mut TRACE_EVENT_INFO = unsafe { mem::transmute(&mut [0u8; BUF_SIZE]) };
        let mut result = unsafe { TdhGetEventInformation( self.0, None, Some(event_info as *mut TRACE_EVENT_INFO), &mut buffer_size) };
        if result == ERROR_INSUFFICIENT_BUFFER.0 {
            event_info = unsafe { mem::transmute(vec![0u8; buffer_size as usize].as_ptr()) };
            result = unsafe { TdhGetEventInformation( self.0, None, Some(event_info as *mut TRACE_EVENT_INFO), &mut buffer_size) };
        }
        if result != ERROR_SUCCESS.0  {
            return write!(f, "Failed to TdhGetEventInformation {result} buffer_size: {buffer_size}");
        }
        #[inline]
        fn is_string_event(flag: u16) -> bool {
            (flag & EVENT_HEADER_FLAG_STRING_ONLY as u16) != 0
        }
        #[inline]
        fn u16cstr_from_slice_with_offset(slice: &[u8] , offset: u32) -> Option<&U16CStr>{
            if offset > 0 {
                U16CStr::from_slice_truncate(cast_slice_truncate(&slice[(offset as usize)..])).ok()
            } else {
                None
            }
        }
        let event_info_slice = unsafe { slice::from_raw_parts(event_info as *const TRACE_EVENT_INFO as *const u8, buffer_size as usize) };

        let provider_id = &header.ProviderId;
        let provider_name = u16cstr_from_slice_with_offset(event_info_slice, event_info.ProviderNameOffset).unwrap_or_default();
        let level_name = u16cstr_from_slice_with_offset(event_info_slice, event_info.LevelNameOffset).unwrap_or_default();
        let channel_name = u16cstr_from_slice_with_offset(event_info_slice, event_info.ChannelNameOffset).unwrap_or_default();
        let keywords_name = u16cstr_from_slice_with_offset(event_info_slice, event_info.KeywordsNameOffset).unwrap_or_default();
        let event_name =  {
            let event_name_offset = unsafe { event_info.Anonymous1.EventNameOffset };
            if event_name_offset != 0 {
                u16cstr_from_slice_with_offset(event_info_slice, event_name_offset).unwrap_or_default()
            } else {
                u16cstr_from_slice_with_offset(event_info_slice, event_info.TaskNameOffset).unwrap_or_default()
            }
        };
        let opcode_name = u16cstr_from_slice_with_offset(event_info_slice, event_info.OpcodeNameOffset).unwrap_or_default();
        let event_message = u16cstr_from_slice_with_offset(event_info_slice, event_info.EventMessageOffset).unwrap_or_default();
        let provider_message = u16cstr_from_slice_with_offset(event_info_slice, event_info.ProviderMessageOffset).unwrap_or_default();

        let mut arr = vec![];
        let mut user_string = U16CString::new();
        if is_string_event(header.Flags) {
            user_string = unsafe { U16CStr::from_ptr_truncate(self.0.UserData as *const u16, (self.0.UserDataLength / 2) as usize).unwrap_or_default().to_owned() };
        } else {
            let event_property_info_array = unsafe { slice::from_raw_parts(event_info.EventPropertyInfoArray.as_ptr(), event_info.PropertyCount as usize) };
            let mut i = 0usize;
            while i < event_info.PropertyCount as usize {
                let event_property_info = &event_property_info_array[i];
                let offset = event_property_info.NameOffset;
                let filed_name = u16cstr_from_slice_with_offset(event_info_slice, offset).unwrap_or_default();
                arr.push(filed_name);
                i = i + 1;
            }
        }

        write!(f, "{0}/{1}  {2}
                   provider_id: {provider_id:?} 
                   provider_name: {provider_name:?}
                   level_name: {level_name:?}
                   channel_name: {channel_name:?}
                   keywords_name: {keywords_name:?}
                   {event_name:?}/{opcode_name:?}
                   event_message: {event_message:?}
                   provider_message: {provider_message:?}
                   ProcessId: {:?} ThreadId: {} {}
                   {arr:?}
                   {user_string:?}",
                   header.ProcessId as i32, header.ThreadId as i32, dt_local,)
    }
}
