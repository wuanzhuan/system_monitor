use std::{
    ffi, fmt, mem, ptr, slice,
    sync::{
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::third_extend::strings::*;
use chrono::*;
use event_decoder::*;
use lazy_static::lazy_static;
use tracing::{error, info, warn};
use widestring::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::System::Diagnostics::Etw::*,
    Win32::System::SystemInformation::*,
};

mod event_decoder;
mod event_kernel;

const SESSION_NAME_SYSMON: &U16CStr = u16cstr!("sysmonx");
const SESSION_NAME_NT: &U16CStr = u16cstr!("NT Kernel Logger");
const INVALID_PROCESSTRACE_HANDLE: u64 = if cfg!(target_pointer_width = "64") {
    0xffffffff_ffffffff
} else {
    0x00000000_ffffffff
};

// {ADA6BC38-93C9-00D1-7462-11D6841904AA}
const DUMMY_GUID: GUID = GUID {
    data1: 0xADA6BC38,
    data2: 0x93C9,
    data3: 0x00D1,
    data4: [0x74, 0x62, 0x11, 0xD6, 0x84, 0x19, 0x04, 0xAA],
};

#[repr(C)]
struct EtwPropertiesBuf(EVENT_TRACE_PROPERTIES, [u8]);
struct ConfigKernel {
    is_selected: bool,
    event_desc: &'static event_kernel::EventsDescribe,
}

pub struct Controller {
    h_trace_session: CONTROLTRACE_HANDLE,
    h_trace_consumer: PROCESSTRACE_HANDLE,
    h_consumer_thread: Option<thread::JoinHandle<()>>,
    config: Vec<ConfigKernel>,
}

pub type FnCompletion = fn(Result<()>);

struct EventRecord<'a>(&'a EVENT_RECORD);

lazy_static! {
    static ref CONTEXT: Arc::<Mutex<Controller>> = Arc::new(Mutex::new(Controller::new()));
}

impl Controller {
    fn new() -> Self {
        let mut cxt = Self {
            h_trace_session: CONTROLTRACE_HANDLE::default(),
            h_trace_consumer: PROCESSTRACE_HANDLE {
                Value: INVALID_PROCESSTRACE_HANDLE,
            },
            h_consumer_thread: None,
            config: Vec::<ConfigKernel>::new(),
        };
        for item in event_kernel::EVENTS_DESC.iter() {
            cxt.config.push(ConfigKernel {
                is_selected: false,
                event_desc: item,
            });
        }
        cxt
    }

    pub fn start(fn_completion: FnCompletion) -> Result<()> {
        let context_arc = CONTEXT.clone();
        let mut context_mg = context_arc
            .try_lock()
            .map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?;
        unsafe {
            let mut h_trace = CONTROLTRACE_HANDLE::default();
            let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
            let session_name: &U16CStr = if is_win8_or_greater {
                SESSION_NAME_SYSMON
            } else {
                SESSION_NAME_NT
            };
            let mut properties_buf = make_properties(is_win8_or_greater, session_name);

            let mut error: Result<()>;
            loop {
                error = StartTraceW(
                    &mut h_trace,
                    session_name.as_pcwstr(),
                    &mut properties_buf.0,
                );
                match error {
                    Ok(_) => {
                        context_mg.h_trace_session = h_trace;
                        break;
                    }
                    Err(e) => {
                        if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                            let error = ControlTraceW(
                                CONTROLTRACE_HANDLE::default(),
                                session_name.as_pcwstr(),
                                &mut properties_buf.0,
                                EVENT_TRACE_CONTROL_STOP,
                            );
                            if error.is_ok() {
                                warn!("The {session_name:#?} is already exist. and stop before restart");
                                continue;
                            }
                            error!(
                                "The {session_name:#?} is already exist. And failed to stop: {:#?}",
                                error
                            );
                            return error;
                        }
                        error!("Failed to StartTraceW: {:#?}", e);
                        return Err(e);
                    }
                }
            }

            context_mg.update_config()?;

            unsafe extern "system" fn callback(eventrecord: *mut EVENT_RECORD) {
                let er = mem::transmute(eventrecord);
                info!("{}", EventRecord(er));
            }

            let mut trace_log = EVENT_TRACE_LOGFILEW {
                Context: &mut *context_mg as *mut Controller as *mut ffi::c_void,
                LoggerName: PWSTR::from_raw(session_name.as_ptr() as *mut u16),
                Anonymous1: EVENT_TRACE_LOGFILEW_0 {
                    ProcessTraceMode: PROCESS_TRACE_MODE_EVENT_RECORD
                        | PROCESS_TRACE_MODE_REAL_TIME,
                },
                Anonymous2: EVENT_TRACE_LOGFILEW_1 {
                    EventRecordCallback: Some(callback),
                },
                ..Default::default()
            };
            let h_consumer = OpenTraceW(&mut trace_log);
            if INVALID_PROCESSTRACE_HANDLE == h_consumer.Value {
                return Err(Error::from_win32());
            }
            context_mg.h_trace_consumer = h_consumer;

            let (tx, rx) = mpsc::channel::<Error>();
            let h_thread = thread::spawn(move || {
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
            } else {
                let e = r_recv.unwrap();
                error!("{}", e);
                return Err(e);
            }
        }
    }

    pub fn stop() -> Result<()> {
        let context_arc = CONTEXT.clone();
        let mut context_mg = context_arc
            .try_lock()
            .map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?;
        unsafe {
            if 0 != context_mg.h_trace_session.Value {
                let is_win8_or_greater = GetVersion() >= _WIN32_WINNT_WINBLUE;
                let session_name: &U16CStr = if is_win8_or_greater {
                    SESSION_NAME_SYSMON
                } else {
                    SESSION_NAME_NT
                };
                let mut properties_buf = make_properties(is_win8_or_greater, session_name);
                let error = ControlTraceW(
                    context_mg.h_trace_session,
                    session_name.as_pcwstr(),
                    &mut properties_buf.0,
                    EVENT_TRACE_CONTROL_STOP,
                );
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

    fn update_config(&self) -> Result<()> {
        #[derive(Default)]
        struct PerfInfoGroupMask {
            masks: [u32; 8],
        }
        let mut gm = PerfInfoGroupMask::default();
        gm.masks[0] = EVENT_TRACE_FLAG_PROCESS.0;
        for item in self.config.iter() {
            if !item.is_selected {
                continue;
            }
            gm.masks[(item.event_desc.major.flag >> 32) as usize] |=
                item.event_desc.major.flag as u32;
        }
        unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceSystemTraceEnableFlagsInfo,
                ptr::addr_of!(gm.masks) as *const ffi::c_void,
                std::mem::size_of_val(&gm.masks) as u32,
            )?;
            let mut vec_event_id = Vec::<CLASSIC_EVENT_ID>::with_capacity(32);
            for item in self.config.iter() {
                if !item.is_selected {
                    continue;
                }
                for item_minor in item.event_desc.minors.iter() {
                    let mut id = CLASSIC_EVENT_ID::default();
                    id.EventGuid = item.event_desc.guid;
                    id.Type = item_minor.op_code as u8;
                    vec_event_id.push(id);
                }
            }

            TraceSetInformation(
                self.h_trace_session,
                TraceStackTracingInfo,
                vec_event_id.as_ptr() as *const ffi::c_void,
                vec_event_id.len() as u32,
            )?;
        }
        Ok(())
    }
}

fn make_properties(is_win8_or_greater: bool, session_name: &U16CStr) -> Box<EtwPropertiesBuf> {
    let properties_buf_len = mem::size_of::<EVENT_TRACE_PROPERTIES>() + session_name.len() * 2 + 2;
    let properties_buf = vec![0u8; properties_buf_len].leak() as *mut [u8] as *mut EtwPropertiesBuf;
    unsafe {
        let mut properties_buf = Box::from_raw(properties_buf);
        let properties = &mut (*properties_buf).0;
        properties.EnableFlags = EVENT_TRACE_FLAG_PROCESS;
        properties.Wnode.BufferSize = properties_buf_len as u32;
        properties.Wnode.Guid = if is_win8_or_greater {
            DUMMY_GUID
        } else {
            SystemTraceControlGuid
        };
        properties.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        properties.Wnode.ClientContext = 1;
        properties.FlushTimer = 1;
        properties.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE
            | EVENT_TRACE_REAL_TIME_MODE
            | EVENT_TRACE_USE_LOCAL_SEQUENCE;
        properties.LoggerNameOffset = mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

        properties_buf
    }
}

impl<'a> fmt::Display for EventRecord<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = event_decoder::Decoder::new(self.0);
        match r {
            Ok(mut decoder) => {
                let r = decoder.decode();
                match r {
                    Ok(event_record_decoded) => {
                        write!(f,"{:?}", event_record_decoded)
                    },
                    Err(e) => {
                        write!(f,"{:?}", e)
                    }
                }
            },
            Err(e) => {
                write!(f,"{:?}", e)
            }
        }
    }
}
