use std::{
    ffi, fmt, mem, ptr,
    rc::Rc,
    sync::{
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use crate::third_extend::strings::*;
use lazy_static::lazy_static;
use tracing::{error, warn};
use widestring::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::System::Diagnostics::Etw::*,
    Win32::System::SystemInformation::*,
};

pub use event_kernel::EVENTS_DESC;

mod event_decoder;
mod event_kernel;
mod event_config;

pub use event_decoder::EventRecordDecoded;


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

lazy_static! {
    static ref CONTEXT: Arc::<Mutex<Controller>> = Arc::new(Mutex::new(Controller::new()));
}

#[repr(C)]
struct EtwPropertiesBuf(EVENT_TRACE_PROPERTIES, [u8]);

struct EventRecord<'a>(&'a EVENT_RECORD);

pub struct Controller {
    config: event_config::Config,
    h_trace_session: CONTROLTRACE_HANDLE,
    h_trace_consumer: PROCESSTRACE_HANDLE,
    h_consumer_thread: Option<thread::JoinHandle<()>>,
    is_win8_or_greater: bool,
    event_record_callback: Option<Rc<dyn Fn(EventRecordDecoded)>>,
}

unsafe impl std::marker::Send for Controller{}

impl Controller {
    fn new() -> Self {
        let cxt = Self {
            config: event_config::Config::new(event_kernel::EVENTS_DESC),
            h_trace_session: CONTROLTRACE_HANDLE::default(),
            h_trace_consumer: PROCESSTRACE_HANDLE {
                Value: INVALID_PROCESSTRACE_HANDLE,
            },
            h_consumer_thread: None,
            is_win8_or_greater: unsafe{ GetVersion() } >= _WIN32_WINNT_WINBLUE,
            event_record_callback: None,
        };
        cxt
    }

    pub fn start(fn_event_callback: impl Fn(EventRecordDecoded) + Send + 'static, fn_completion: impl FnOnce(Result<()>) + Send + 'static) -> Result<()> {
        let context_arc = CONTEXT.clone();
        let mut context_mg = context_arc
            .try_lock()
            .map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?;
        let mut h_trace = CONTROLTRACE_HANDLE::default();
        let session_name: &U16CStr = if context_mg.is_win8_or_greater {
            SESSION_NAME_SYSMON
        } else {
            SESSION_NAME_NT
        };
        let mut properties_buf = make_properties(context_mg.is_win8_or_greater, session_name);

        let r = loop {
            loop {
                let r = unsafe{
                    StartTraceW(
                        &mut h_trace,
                        session_name.as_pcwstr(),
                        &mut properties_buf.0,
                    )
                };
                match r {
                    Ok(_) => {
                        context_mg.h_trace_session = h_trace;
                        break Ok(());
                    }
                    Err(e) => {
                        if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                            if let Err(e) = unsafe{
                                ControlTraceW(
                                    CONTROLTRACE_HANDLE::default(),
                                    session_name.as_pcwstr(),
                                    &mut properties_buf.0,
                                    EVENT_TRACE_CONTROL_STOP,
                                )
                            } {
                                error!("The {session_name:#?} is already exist. And failed to stop: {:#?}", e);
                                break Err(e);
                            };
                            warn!("The {session_name:#?} is already exist. and stop before restart");
                            continue;
                        }
                        error!("Failed to StartTraceW: {:#?}", e);
                        break Err(e);
                    }
                }
            }?;

            if let Err(e) = context_mg.update_config() {
                break Err(e);
            }
    
            context_mg.event_record_callback = Some(Rc::new(fn_event_callback));
            let mut trace_log = EVENT_TRACE_LOGFILEW {
                Context: &mut *context_mg as *mut Controller as *mut ffi::c_void,
                LoggerName: PWSTR::from_raw(session_name.as_ptr() as *mut u16),
                Anonymous1: EVENT_TRACE_LOGFILEW_0 {
                    ProcessTraceMode: PROCESS_TRACE_MODE_EVENT_RECORD
                        | PROCESS_TRACE_MODE_REAL_TIME,
                },
                Anonymous2: EVENT_TRACE_LOGFILEW_1 {
                    EventRecordCallback: Some(Controller::callback),
                },
                ..Default::default()
            };
            let h_consumer = unsafe{ OpenTraceW(&mut trace_log) };
            if INVALID_PROCESSTRACE_HANDLE == h_consumer.Value {
                context_mg.event_record_callback = None;
                let e = Err(Error::from_win32());
                error!("Failed to OpenTraceW: {:#?}", e);
                break e;
            }
            context_mg.h_trace_consumer = h_consumer;
    
            let (tx, rx) = mpsc::channel::<Error>();
            let h_thread = thread::spawn(move || {
                let ft_now = unsafe{ GetSystemTimeAsFileTime() };
                let r_pt = unsafe{ ProcessTrace(&[h_consumer], Some(&ft_now), None) };
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
            match r_recv {
                Err(e) => {
                    if e == RecvTimeoutError::Timeout {
                        context_mg.h_consumer_thread = Some(h_thread);
                        break Ok(());
                    }
                    error!("Failed to recv_timeout {}", e);
                    context_mg.h_consumer_thread = None;
                    break Err(E_FAIL.into());
                },
                Ok(e) => {
                    error!("{}", e);
                    context_mg.h_consumer_thread = None;
                    break Err(e);
                }
            }
        };
        if r.is_err() {
            let _ = Self::stop(Some(context_mg));
        }
        r
    }

    pub fn stop(mg: Option<MutexGuard<Controller>>) -> Result<()> {
        let context_arc = CONTEXT.clone();
        let mut context_mg = mg.unwrap_or(context_arc
            .try_lock()
            .map_err(|_| ERROR_LOCK_VIOLATION.to_hresult())?);

        if 0 != context_mg.h_trace_session.Value {
            let session_name: &U16CStr = if context_mg.is_win8_or_greater {
                SESSION_NAME_SYSMON
            } else {
                SESSION_NAME_NT
            };
            let mut properties_buf = make_properties(context_mg.is_win8_or_greater, session_name);
            let error = unsafe{
                ControlTraceW(
                    context_mg.h_trace_session,
                    session_name.as_pcwstr(),
                    &mut properties_buf.0,
                    EVENT_TRACE_CONTROL_STOP,
                )
            };
            context_mg.h_trace_session.Value = 0;
            if let Err(e) = error {
                error!("failed to ControlTraceW {}", e);
            }
        }

        if INVALID_PROCESSTRACE_HANDLE != context_mg.h_trace_consumer.Value {
            let error = unsafe{ CloseTrace(context_mg.h_trace_consumer) };
            context_mg.h_trace_consumer.Value = INVALID_PROCESSTRACE_HANDLE;
            if let Err(e) = error {
                if ERROR_CTX_CLOSE_PENDING.to_hresult() != e.code() {
                    error!("failed to CloseTrace {}", e);
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

    pub fn set_config_enables(index_major: usize, index_minor: Option<usize>, checked: bool) {
        let context_arc = CONTEXT.clone();
        if let Ok(mut context_mg) = context_arc.lock() {
            let mut is_change = false;
            if let Some(index) = index_minor {
                if context_mg.config.events_enables[index_major].minors[index] != checked {
                    if !context_mg.config.events_enables[index_major].major {
                        context_mg.config.events_enables[index_major].major = true;
                    }
                    is_change = true;
                    context_mg.config.events_enables[index_major].minors[index] = checked;
                }
            } else {
                if context_mg.config.events_enables[index_major].major != checked {
                    is_change = true;
                    context_mg.config.events_enables[index_major].major = checked;
                }
            }
            if is_change && 0 != context_mg.h_trace_session.Value {
                let _ = context_mg.update_config();
            }
        };
    }

    unsafe extern "system" fn callback(eventrecord: *mut EVENT_RECORD) {
        let r = event_decoder::Decoder::new(mem::transmute(eventrecord));
        match r {
            Ok(mut decoder) => {
                let r = decoder.decode();
                match r {
                    Ok(event_record_decoded) => {
                        let context_arc = CONTEXT.clone();
                        if let Ok(context_mg) = context_arc.try_lock() {
                            if let Some(ref cb) = context_mg.event_record_callback {
                                let cb = cb.clone();
                                mem::drop(context_mg);
                                cb(event_record_decoded);
                            }
                        };
                    },
                    Err(e) => {
                        error!("Faild to decode: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Faild to Decoder::new: {}", e);
            }
        }
    }

    fn update_config(&self) -> Result<()> {
        let gm = self.config.get_group_mask();
        if let Err(e) = unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceSystemTraceEnableFlagsInfo,
                ptr::addr_of!(gm.masks) as *const ffi::c_void,
                std::mem::size_of_val(&gm.masks) as u32,
            )
        } {
            error!("Failed to TraceSetInformation TraceSystemTraceEnableFlagsInfo: {}", e);
            return Err(e);
        }
        let (vec_event_id, size) = self.config.get_classic_event_id_vec();
        if let Err(e) = unsafe{ TraceSetInformation(
            self.h_trace_session,
            TraceStackTracingInfo,
            vec_event_id.as_ptr() as *const ffi::c_void,
            size as u32,
        )} {
            error!("Failed to TraceSetInformation TraceStackTracingInfo: {}", e);
            return Err(e);
        }
        Ok(())
    }
}

fn make_properties(is_win8_or_greater: bool, session_name: &U16CStr) -> Box<EtwPropertiesBuf> {
    let properties_buf_len = mem::size_of::<EVENT_TRACE_PROPERTIES>() + session_name.len() * 2 + 2;
    let properties_buf = vec![0u8; properties_buf_len].leak() as *mut [u8] as *mut EtwPropertiesBuf;
    let mut properties_buf = unsafe{ Box::from_raw(properties_buf) };
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

