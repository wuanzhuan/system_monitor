use std::{
    ffi::*,
    mem::*,
    ptr,
    thread,
    sync::{Arc, Mutex}
};
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::System::Diagnostics::Etw::*,
    Win32::System::SystemInformation::*
};
use widestring::*;

mod event_kernel;
pub use event_kernel::*;
use lazy_static::lazy_static;


const SESSION_NAME_SYSMON: &str = "sysmonx\0";
const SESSION_NAME_NT:&str = "NT Kernel Logger\0";
const INVALID_PROCESSTRACE_HANDLE: u64 =  if cfg!(target_pointer_width = "64") { 0xffffffff_ffffffff } else{ 0x00000000_ffffffff };

// {ADA6BC38-93C9-00D1-7462-11D6841904AA}
const DUMMY_GUID: GUID = GUID{ data1: 0xADA6BC38, data2: 0x93C9, data3: 0x00D1, data4: [0x74, 0x62, 0x11, 0xD6, 0x84, 0x19, 0x04, 0xAA] };

#[repr(C)]
struct EtwPropertiesBuf(EVENT_TRACE_PROPERTIES, [u8]);
struct ConfigKernel{
    is_selected: bool,
    event_desc: &'static EventsDescribe
}


pub struct Context{
    h_trace_session: u64,
    h_trace_consumer: u64,
    h_consumer_thread: Option<thread::JoinHandle<u32>>,
    config: Vec::<ConfigKernel>,
}

impl Context{
    fn new() -> Self {
        let mut cxt = Self{h_trace_session: 0, h_trace_consumer: INVALID_PROCESSTRACE_HANDLE, h_consumer_thread: None, config: Vec::<ConfigKernel>::new()};
        for item in EVENTS_DESC.iter() {
            cxt.config.push( ConfigKernel{is_selected: false, event_desc: item});
        };
        cxt
    }

    pub fn start(&mut self) -> Result<()>{
        unsafe {
            if 0 != self.h_trace_session {
                return Ok(())
            }
            let mut h_trace: u64 = 0;
            let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
            let session_name: OsString = if is_win8_or_greater { SESSION_NAME_SYSMON.into() } else { SESSION_NAME_NT.into() };
            let mut properties_buf = make_properties(is_win8_or_greater, session_name.as_os_str());
    
            let mut error: u32;
            loop {
                error = StartTraceW(&mut h_trace, session_name.as_os_str(), &mut properties_buf.0);
                if error == ERROR_SUCCESS.0{
                    self.h_trace_session =  h_trace;
                    break;
                }
                if error != ERROR_ALREADY_EXISTS.0 {
                    break;
                }
                error = ControlTraceW(0u64, session_name.as_os_str(), &mut properties_buf.0, EVENT_TRACE_CONTROL_STOP);
                if error != ERROR_SUCCESS.0 {
                    break;
                }
            }
            if error != ERROR_SUCCESS.0 {
                let code = HRESULT::from(WIN32_ERROR(error));
                return Err(Error::new(code, code.message()));
            }
            let error = self.update_config();
            if error.is_err() {
                return error;
            }

            let wstr_para: Param<PWSTR> = session_name.into_param();
            unsafe extern "system" fn callback(eventrecord: *mut EVENT_RECORD){
                let er = std::mem::transmute(eventrecord);
                println!("{}", EventRecord(er));
            }
    
            let mut trace_log = EVENT_TRACE_LOGFILEW{
                Context: self as *mut Context as *mut c_void,
                LoggerName: wstr_para.abi(),
                Anonymous1: EVENT_TRACE_LOGFILEW_0{ ProcessTraceMode:  PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_REAL_TIME },
                Anonymous2: EVENT_TRACE_LOGFILEW_1{ EventRecordCallback: Some(callback)},
                ..Default::default()
            };
            let h_consumer = OpenTraceW(&mut trace_log);
            if INVALID_PROCESSTRACE_HANDLE == h_consumer {
                return Err(Error::from_win32());
            }
            self.h_trace_consumer = h_consumer;

            let h_thread = thread::spawn(move ||{
                let mut ft_now = FILETIME::default();
                GetSystemTimeAsFileTime(&mut ft_now);
                let error = ProcessTrace(&h_consumer, 1, &ft_now, std::ptr::null());
                error
            });
            self.h_consumer_thread = Some(h_thread);
        }
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()>{
        unsafe{
            if 0 != self.h_trace_session {
                let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
                let session_name: OsString = if is_win8_or_greater { SESSION_NAME_SYSMON.into() } else { SESSION_NAME_NT.into() };
                let mut properties_buf = make_properties(is_win8_or_greater, session_name.as_os_str());
                let error = ControlTraceW(self.h_trace_session, PWSTR::default(), &mut properties_buf.0, EVENT_TRACE_CONTROL_STOP);
                self.h_trace_session = 0;
                if error != ERROR_SUCCESS.0 {
                    //let code = HRESULT::from(WIN32_ERROR(error));
                    //return Err(Error::new(code, code.message()));
                    //log
                }
            }

            if 0 != self.h_trace_consumer {
                let error = CloseTrace(self.h_trace_consumer);
                self.h_trace_consumer = 0;
                if error != ERROR_SUCCESS.0 {
                    //let code = HRESULT::from(WIN32_ERROR(error));
                    //return Err(Error::new(code, code.message()));
                    //log
                }
            }

            if let Some(x) = &self.h_consumer_thread {

            }
        }
        Ok(())
    }

    pub fn update_config(&self)-> Result<()> {
        
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
            let error = TraceSetInformation(self.h_trace_session, TraceSystemTraceEnableFlagsInfo, std::ptr::addr_of!(gm.masks) as *const c_void, std::mem::size_of_val(&gm.masks) as u32);
            if error != ERROR_SUCCESS.0{
                let code = HRESULT::from(WIN32_ERROR(error));
                return Err(Error::new(code, code.message()));
            }
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
    
            let error = TraceSetInformation(self.h_trace_session, TraceStackTracingInfo, vec_event_id.as_ptr() as *const c_void, vec_event_id.len() as u32);
            if error != ERROR_SUCCESS.0{
                let code = HRESULT::from(WIN32_ERROR(error));
                return Err(Error::new(code, code.message()));
            }
        }
        Ok(())
    }
}

fn make_properties(is_win8_or_greater: bool, session_name: &OsStr) -> Box<EtwPropertiesBuf>{
    let properties_buf_len = size_of::<EVENT_TRACE_PROPERTIES>() + session_name.len() * 2;
    let properties_buf = vec![0u8; properties_buf_len].leak() as *mut[u8] as *mut EtwPropertiesBuf;
    unsafe{
        let mut properties_buf = Box::from_raw(properties_buf);
        let mut properties = &mut(*properties_buf).0;
        properties.EnableFlags = EVENT_TRACE_FLAG_PROCESS;
        properties.Wnode.BufferSize = properties_buf_len as u32;
        properties.Wnode.Guid = if is_win8_or_greater { DUMMY_GUID } else { SystemTraceControlGuid };
        properties.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        properties.Wnode.ClientContext = 1;
        properties.FlushTimer = 1;
        properties.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE | EVENT_TRACE_REAL_TIME_MODE | EVENT_TRACE_USE_LOCAL_SEQUENCE;
        properties.LoggerNameOffset = size_of::<EVENT_TRACE_PROPERTIES>() as u32;

        properties_buf
    }
}


struct EventRecord<'a>(&'a EVENT_RECORD);

use std::fmt;
use chrono::*;

impl<'a> fmt::Display for EventRecord<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = &self.0.EventHeader;
        let duration = Utc.ymd(1970, 1, 1) - Utc.ymd(1601, 1, 1);
        let dt_utc = Utc.timestamp_millis(header.TimeStamp / 10 / 1000 - duration.num_milliseconds());
        let dt_local: DateTime<Local> = dt_utc.into();
        const BUF_SIZE: u32 = 4096;
        let mut buffer_size = BUF_SIZE;
        let event_info: *mut TRACE_EVENT_INFO = &mut [0u8; BUF_SIZE as usize] as *mut u8 as *mut TRACE_EVENT_INFO;
        let event_info_buf: *mut u8 = event_info as *mut u8;
        let result = unsafe { TdhGetEventInformation( self.0, 0, ptr::null(), event_info, &mut buffer_size) };
        if result == ERROR_SUCCESS.0  {
            let task_name = unsafe { UCStr::from_ptr_str(event_info_buf.offset((*event_info).TaskNameOffset as isize) as *const u16) };
            let opcode_name = unsafe { UCStr::from_ptr_str(event_info_buf.offset((*event_info).OpcodeNameOffset as isize) as *const u16) };
            write!(f, "{} {}/{} ProcessId: {} ThreadId: {}", dt_local, task_name.display(), opcode_name.display(), header.ProcessId as i32, header.ThreadId as i32)
        } else {
            write!(f, "{} ProcessId: {:+} ThreadId: {:+}", dt_local, header.ProcessId, header.ThreadId)
        }
    }
}

lazy_static!{
    pub static ref CONTEXT: Arc::<Mutex<Context>> = Arc::new(Mutex::new(Context::new()));
}