use crate::{third_extend::strings::*, utils::TimeStamp};
use anyhow::{anyhow, Error as ErrorAnyhow, Result};
use linked_hash_map::LinkedHashMap;
use once_cell::sync::Lazy;
use parking_lot::{FairMutex, FairMutexGuard};
use std::{
    cell::{RefCell, UnsafeCell},
    ffi, fmt, mem, ptr,
    rc::Rc,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};
use tracing::{debug, error, warn};
use widestring::*;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{Diagnostics::Etw::*, Performance::QueryPerformanceCounter, SystemInformation::*},
    },
};

mod event_config;
mod event_decoder;
mod event_kernel;
mod stack_walk;

pub use event_decoder::{EventRecordDecoded, PropertyDecoded};
pub use event_kernel::{event_property::*, EVENTS_DESC, LOST_EVENT_GUID};
pub use stack_walk::StackWalkMap;

const SESSION_NAME_SYSMON: &U16CStr = u16cstr!("sysmonx");
const SESSION_NAME_NT: &U16CStr = u16cstr!("NT Kernel Logger");
const INVALID_PROCESSTRACE_HANDLE: u64 = if cfg!(target_pointer_width = "64") {
    0xffffffff_ffffffff
} else {
    0x00000000_ffffffff
};

// {ADA6BC38-93C9-00D1-7462-11D6841904AA}
const DUMMY_GUID: GUID = GUID::from_u128(0xADA6BC38_93C9_00D1_7462_11D6841904AA);

static CONTEXT: Lazy<FairMutex<Controller>> = Lazy::new(|| FairMutex::new(Controller::new()));

#[repr(C)]
struct EtwPropertiesBuf(EVENT_TRACE_PROPERTIES, [u8]);

pub struct Controller {
    is_stopping: bool,
    config: event_config::Config,
    h_trace_session: CONTROLTRACE_HANDLE,
    h_trace_consumer: PROCESSTRACE_HANDLE,
    h_consumer_thread: Option<thread::JoinHandle<()>>,
    is_win8_or_greater: bool,
    event_record_callback: Option<
        Rc<
            UnsafeCell<
                dyn FnMut(
                    EventRecordDecoded,
                    /*stack_walk*/ Option<StackWalk>,
                    /*is_selected*/ bool,
                ),
            >,
        >,
    >,
    unstored_events_map: RefCell<StackWalkMap<()>>,
    boot_time: TimeStamp,
    perf_freq: i64,
}

unsafe impl std::marker::Send for Controller {}

impl Controller {
    fn new() -> Self {
        let cxt = Self {
            is_stopping: false,
            config: event_config::Config::new(event_kernel::EVENTS_DESC),
            h_trace_session: CONTROLTRACE_HANDLE::default(),
            h_trace_consumer: PROCESSTRACE_HANDLE {
                Value: INVALID_PROCESSTRACE_HANDLE,
            },
            h_consumer_thread: None,
            is_win8_or_greater: unsafe { GetVersion() } >= _WIN32_WINNT_WINBLUE,
            event_record_callback: None,
            unstored_events_map: RefCell::new(StackWalkMap::new(32, 10, 15)),
            boot_time: TimeStamp(0),
            perf_freq: 1000_0000,
        };
        cxt
    }

    pub fn start(
        fn_event_callback: impl FnMut(
                EventRecordDecoded,
                /*stack_walk*/ Option<StackWalk>,
                /*is_enabled*/ bool,
            ) + Send
            + 'static,
        fn_completion: impl FnOnce(Result<()>) + Send + 'static,
    ) -> Result<()> {
        let mut context_mg = CONTEXT.lock();
        context_mg.is_stopping = false;
        let mut h_trace = CONTROLTRACE_HANDLE::default();
        let session_name: &U16CStr = if context_mg.is_win8_or_greater {
            SESSION_NAME_SYSMON
        } else {
            SESSION_NAME_NT
        };
        let mut properties_buf = make_properties(context_mg.is_win8_or_greater, session_name);

        let r = loop {
            loop {
                let r = unsafe {
                    StartTraceW(
                        &mut h_trace,
                        session_name.as_pcwstr(),
                        &mut properties_buf.0,
                    )
                };
                if r == ERROR_SUCCESS {
                    context_mg.h_trace_session = h_trace;
                    break Ok(());
                } else {
                    if r == ERROR_ALREADY_EXISTS {
                        let r = unsafe {
                            ControlTraceW(
                                CONTROLTRACE_HANDLE::default(),
                                session_name.as_pcwstr(),
                                &mut properties_buf.0,
                                EVENT_TRACE_CONTROL_STOP,
                            )
                        };
                        if r == ERROR_SUCCESS {
                            warn!(
                                "The {session_name:#?} is already exist. and stop before restart"
                            );
                            continue;
                        }
                        break Err(anyhow!(
                            "The {session_name:#?} is already exist. And failed to stop: {r:#?}"
                        ));
                    }
                    break Err(anyhow!("Failed to StartTraceW: {r:#?}"));
                }
            }?;

            if let Err(e) = context_mg.update_config() {
                break Err(e);
            }

            context_mg.event_record_callback = Some(Rc::new(UnsafeCell::new(fn_event_callback)));
            let mut trace_log = EVENT_TRACE_LOGFILEW {
                Context: &mut *context_mg as *mut Controller as *mut ffi::c_void,
                LoggerName: PWSTR::from_raw(session_name.as_ptr() as *mut u16),
                Anonymous1: EVENT_TRACE_LOGFILEW_0 {
                    ProcessTraceMode: PROCESS_TRACE_MODE_EVENT_RECORD
                        | PROCESS_TRACE_MODE_REAL_TIME
                        | PROCESS_TRACE_MODE_RAW_TIMESTAMP,
                },
                Anonymous2: EVENT_TRACE_LOGFILEW_1 {
                    EventRecordCallback: Some(Controller::unsafe_event_record_callback),
                },
                ..Default::default()
            };
            let h_consumer = unsafe { OpenTraceW(&mut trace_log) };
            if INVALID_PROCESSTRACE_HANDLE == h_consumer.Value {
                context_mg.event_record_callback = None;
                break Err(anyhow!("Failed to OpenTraceW: {:#?}", Error::from_win32()));
            }
            context_mg.h_trace_consumer = h_consumer;
            context_mg.boot_time = TimeStamp(trace_log.LogfileHeader.BootTime);
            context_mg.perf_freq = trace_log.LogfileHeader.PerfFreq;
            drop(context_mg);

            let (tx, rx) = mpsc::channel::<ErrorAnyhow>();
            let h_thread = thread::spawn(move || {
                let mut start_time = 0i64;
                let _ = unsafe { QueryPerformanceCounter(&mut start_time) };
                let start_time_ft = TimeStamp(start_time).to_filetime();
                // not set start_time. there is no stackwalk for starting events(> 200)
                // the start_time need to match qpc / systemtime
                let r = unsafe { ProcessTrace(&[h_consumer], Some(&start_time_ft), None) };
                let r = if r == ERROR_SUCCESS {
                    Ok(())
                } else {
                    let msg = format!("Failed to ProcessTrace: {r:#?}");
                    let r_send = tx.send(anyhow!("{msg}"));
                    if r_send.is_ok() {
                        return;
                    }
                    Err(anyhow!("Failed to send {msg}"))
                };
                CONTEXT.lock().h_consumer_thread = None;
                fn_completion(r);
            });
            let r_recv = rx.recv_timeout(Duration::from_millis(200));
            match r_recv {
                Err(e) => {
                    if e == RecvTimeoutError::Timeout {
                        CONTEXT.lock().h_consumer_thread = Some(h_thread);
                        break Ok(());
                    }
                    error!("Failed to recv_timeout {}", e);
                    CONTEXT.lock().h_consumer_thread = None;
                    break Err(e.into());
                }
                Ok(e) => {
                    CONTEXT.lock().h_consumer_thread = None;
                    break Err(e);
                }
            }
        };
        if r.is_err() {
            let _ = Self::stop();
        }
        r
    }

    pub fn stop() -> Result<()> {
        let mut context_mg = CONTEXT.lock();
        context_mg.is_stopping = true;

        if 0 != context_mg.h_trace_session.Value {
            let session_name: &U16CStr = if context_mg.is_win8_or_greater {
                SESSION_NAME_SYSMON
            } else {
                SESSION_NAME_NT
            };
            let h_trace_session = context_mg.h_trace_session;
            let is_win8_or_greater = context_mg.is_win8_or_greater;
            let mut properties_buf = make_properties(is_win8_or_greater, session_name);
            drop(context_mg);
            let error = unsafe {
                ControlTraceW(
                    h_trace_session,
                    session_name.as_pcwstr(),
                    &mut properties_buf.0,
                    EVENT_TRACE_CONTROL_STOP,
                )
            };
            context_mg = CONTEXT.lock();
            context_mg.h_trace_session.Value = 0;
            if error != ERROR_SUCCESS {
                error!("failed to ControlTraceW {error:#?}");
            }
        }

        if INVALID_PROCESSTRACE_HANDLE != context_mg.h_trace_consumer.Value {
            let h_trace_consumer = context_mg.h_trace_consumer;
            drop(context_mg);
            let error = unsafe { CloseTrace(h_trace_consumer) };
            context_mg = CONTEXT.lock();
            context_mg.h_trace_consumer.Value = INVALID_PROCESSTRACE_HANDLE;
            if error != ERROR_SUCCESS {
                if error != ERROR_CTX_CLOSE_PENDING {
                    error!("failed to CloseTrace {error:#?}");
                }
            }
        }
        if context_mg.h_consumer_thread.is_some() {
            let h = context_mg.h_consumer_thread.take().unwrap();
            mem::drop(context_mg);
            let _ = h.join();
            context_mg = CONTEXT.lock();
        }

        // clear other
        let _ = context_mg.event_record_callback.take();
        context_mg.unstored_events_map.borrow_mut().clear();

        Ok(())
    }

    pub fn set_config_enables(index_major: usize, index_minor: Option<usize>, checked: bool) {
        let mut context_mg = CONTEXT.lock();
        let is_change = context_mg
            .config
            .set_enable(index_major, index_minor, checked);
        if is_change && 0 != context_mg.h_trace_session.Value {
            let _ = context_mg.update_config();
        }
    }

    unsafe extern "system" fn unsafe_event_record_callback(event_record: *mut EVENT_RECORD) {
        let er: &mut EVENT_RECORD = mem::transmute(event_record);
        Self::event_record_callback(er)
    }

    #[inline]
    fn event_record_callback(event_record: &mut EVENT_RECORD) {
        let is_stack_walk = event_record.EventHeader.ProviderId == event_kernel::STACK_WALK_GUID;
        let is_module_event = event_record.EventHeader.ProviderId == ImageLoadGuid
            || event_record.EventHeader.ProviderId == ProcessGuid;
        let is_lost_event = event_record.EventHeader.ProviderId == LOST_EVENT_GUID;
        let is_auto_generated = event_record.EventHeader.ProviderId == EventTraceGuid;

        let context_mg = CONTEXT.lock();
        if context_mg.is_stopping {
            return;
        }
        event_record.EventHeader.TimeStamp = TimeStamp::from_qpc(
            event_record.EventHeader.TimeStamp as u64,
            context_mg.boot_time,
            context_mg.perf_freq,
        )
        .0;

        let event_indexes = match get_event_indexes(event_record, Some(&context_mg)) {
            Ok(indexes) => indexes,
            Err(e) => {
                error!("{e}");
                if !is_stack_walk {
                    context_mg.unstored_events_map.borrow_mut().insert(
                        (
                            event_record.EventHeader.ThreadId,
                            event_record.EventHeader.TimeStamp,
                        ),
                        (),
                        format!(
                            "{:?}-{:?} in unstored_events_map",
                            event_record.EventHeader.ProviderId,
                            event_record.EventHeader.EventDescriptor.Opcode
                        ),
                    );
                }
                return;
            }
        };

        // filter non stack walk events
        let mut is_enabled = false;
        if !is_stack_walk {
            let event_enable = context_mg.config.get_enable(event_indexes.0);
            if event_enable.major {
                if event_enable.minors[event_indexes.1] {
                    is_enabled = true;
                }
            } else {
                // the major event is filter by flag. so a error happens when a event that is not enable comes
                // the EventTrace Process Image event is always enable.
                if !is_module_event && !is_auto_generated {
                    if !context_mg
                        .config
                        .is_flag_enable(EVENTS_DESC[event_indexes.0].major.flag)
                    {
                        error!(
                            "No enable major event is coming: {}-{} event_record: {}",
                            EVENTS_DESC[event_indexes.0].major.name,
                            EVENTS_DESC[event_indexes.0].minors[event_indexes.1].name,
                            EventRecord(event_record)
                        );
                    }
                }
            }
            if !is_enabled {
                // escape event
                if !is_module_event && !is_lost_event {
                    context_mg.unstored_events_map.borrow_mut().insert(
                        (
                            event_record.EventHeader.ThreadId,
                            event_record.EventHeader.TimeStamp,
                        ),
                        (),
                        format!(
                            "{:?}-{:?} unstored_events_map",
                            event_record.EventHeader.ProviderId,
                            event_record.EventHeader.EventDescriptor.Opcode
                        ),
                    );
                    return;
                }
            }
        };
        drop(context_mg);

        let mut event_record_decoded = match event_decoder::Decoder::new(event_record) {
            Ok(mut decoder) => match decoder.decode() {
                Ok(event_record_decoded) => event_record_decoded,
                Err(e) => {
                    warn!(
                        "Faild to decode: {e} EventRecord: {}",
                        EventRecord(event_record)
                    );
                    event_decoder::decode_kernel_event(
                        event_record,
                        event_kernel::EVENTS_DESC[event_indexes.0].major.name,
                        event_kernel::EVENTS_DESC[event_indexes.0].minors[event_indexes.1].name,
                    )
                }
            },
            Err(e) => {
                warn!(
                    "Faild to Decoder::new: {e} EventRecord: {}",
                    EventRecord(event_record)
                );
                event_decoder::decode_kernel_event(
                    event_record,
                    event_kernel::EVENTS_DESC[event_indexes.0].major.name,
                    event_kernel::EVENTS_DESC[event_indexes.0].minors[event_indexes.1].name,
                )
            }
        };

        if let Some(display_name) = EVENTS_DESC[event_indexes.0].major.display_name {
            event_record_decoded.set_event_display_name(display_name);
        }

        let context_mg = CONTEXT.lock();
        if is_stack_walk {
            let mut sw = StackWalk::from_event_record_decoded(&event_record_decoded);
            sw.event_timestamp = TimeStamp::from_qpc(
                sw.event_timestamp as u64,
                context_mg.boot_time,
                context_mg.perf_freq,
            )
            .0;

            let removed_option = context_mg.unstored_events_map.borrow_mut().remove(
                &(sw.stack_thread, sw.event_timestamp),
                event_record.EventHeader.TimeStamp,
            );
            if let Some(removed) = removed_option {
                debug!(
                    "The unstored event: {}:{}:{} 's stack walk: {}:{}:{} has removed {}",
                    sw.stack_process,
                    sw.stack_thread as i32,
                    TimeStamp(sw.event_timestamp).to_string_detail(),
                    event_record.EventHeader.ProcessId as i32,
                    event_record.EventHeader.ThreadId as i32,
                    TimeStamp(event_record.EventHeader.TimeStamp).to_string_detail(),
                    removed.0 .1
                );
            } else {
                let cb = context_mg.event_record_callback.clone().unwrap();
                mem::drop(context_mg);
                let cb = unsafe { &mut *cb.get() };
                cb(event_record_decoded, Some(sw), false);
            }
        } else {
            if !is_lost_event {
                let cb = context_mg.event_record_callback.clone().unwrap();
                mem::drop(context_mg);
                let cb = unsafe { &mut *cb.get() };
                cb(event_record_decoded, None, is_enabled);
            } else {
                warn!("Lost_Event: {:#?}", event_record_decoded);
            }
        }

        fn get_event_indexes(
            er: &EVENT_RECORD,
            context_mg_op: Option<&FairMutexGuard<Controller>>,
        ) -> Result<(/*major_index*/ usize, /*minor_index*/ usize)> {
            fn get(
                er: &EVENT_RECORD,
                context_mg: &FairMutexGuard<Controller>,
            ) -> Result<(/*major_index*/ usize, /*minor_index*/ usize)> {
                if let Some((major_index, minor_index)) =
                    context_mg.config.events_opcode_map.get(&(
                        er.EventHeader.ProviderId,
                        er.EventHeader.EventDescriptor.Opcode as u32,
                    ))
                {
                    Ok((*major_index, *minor_index))
                } else {
                    Err(anyhow!(
                        "Failed to find event in events_opcode_map EventRecord: {}",
                        EventRecord(er)
                    ))
                }
            }

            if let Some(context_mg) = context_mg_op {
                get(er, context_mg)
            } else {
                let context_mg = CONTEXT.lock();
                get(er, &context_mg)
            }
        }
    }

    fn update_config(&self) -> Result<()> {
        let gm = self.config.get_group_mask();
        let r = unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceSystemTraceEnableFlagsInfo,
                ptr::addr_of!(gm.masks) as *const ffi::c_void,
                mem::size_of_val(&gm.masks) as u32,
            )
        };
        if r != ERROR_SUCCESS {
            return Err(anyhow!(
                "Failed to TraceSetInformation TraceSystemTraceEnableFlagsInfo: {r:#?}"
            ));
        }
        let (vec_event_id, size) = self.config.get_classic_event_id_vec();
        let r = unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceStackTracingInfo,
                vec_event_id.as_ptr() as *const ffi::c_void,
                size as u32,
            )
        };
        if r != ERROR_SUCCESS {
            return Err(anyhow!(
                "Failed to TraceSetInformation TraceStackTracingInfo: {r:#?}"
            ));
        }
        Ok(())
    }
}

fn make_properties(is_win8_or_greater: bool, session_name: &U16CStr) -> Box<EtwPropertiesBuf> {
    let properties_buf_len = mem::size_of::<EVENT_TRACE_PROPERTIES>() + session_name.len() * 2 + 2;
    let properties_buf = vec![0u8; properties_buf_len].leak() as *mut [u8] as *mut EtwPropertiesBuf;
    let mut properties_buf = unsafe { Box::from_raw(properties_buf) };
    let properties = &mut (*properties_buf).0;
    properties.EnableFlags = EVENT_TRACE_FLAG_NO_SYSCONFIG;
    properties.Wnode.BufferSize = properties_buf_len as u32;
    properties.Wnode.Guid = if is_win8_or_greater {
        DUMMY_GUID
    } else {
        SystemTraceControlGuid
    };
    properties.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
    // if 1 and clear PROCESS_TRACE_MODE_RAW_TIMESTAMP of EVENT_TRACE_LOGFILEA, the StackWalk's event_timestamp is qpc yet.
    // if 1 and set PROCESS_TRACE_MODE_RAW_TIMESTAMP, no event coming in windows 11. because of the StartTime of ProcessTrace
    properties.Wnode.ClientContext = 1;
    properties.BufferSize = 256 * 1024;
    properties.FlushTimer = 1;
    properties.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE
        | EVENT_TRACE_REAL_TIME_MODE
        | EVENT_TRACE_USE_LOCAL_SEQUENCE;
    properties.LoggerNameOffset = mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

    properties_buf
}

struct EventRecord<'a>(&'a EVENT_RECORD);

impl<'a> fmt::Display for EventRecord<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = self.0.EventHeader;
        write!(
            f,
            "\n header:
            Size: {}
            HeaderType: {}
            Flags: {}
            EventProperty: {}
            ThreadId: {}
            ProcessId: {}
            TimeStamp: {}
            ProviderId: {:?}
            event descroptor:
                Id: {}
                Version: {}
                Channel: {}
                Level: {}
                Opcode: {}
                Task: {}
                Keyword: {}
            ActivityId: {:?}",
            header.Size,
            header.HeaderType,
            header.Flags,
            header.EventProperty,
            header.ThreadId as i32,
            header.ProcessId as i32,
            header.TimeStamp,
            header.ProviderId,
            header.EventDescriptor.Id,
            header.EventDescriptor.Version,
            header.EventDescriptor.Channel,
            header.EventDescriptor.Level,
            header.EventDescriptor.Opcode,
            header.EventDescriptor.Task,
            header.EventDescriptor.Keyword,
            header.ActivityId
        )
    }
}

pub static EVENTS_DISPLAY_NAME_MAP: Lazy<
    LinkedHashMap<
        String,
        (
            /*major index*/ usize,
            LinkedHashMap<String, /*minor index*/ usize>,
        ),
    >,
> = Lazy::new(|| {
    let mut map = LinkedHashMap::new();
    for (index, event_desc) in EVENTS_DESC.iter().enumerate() {
        let mut minor_map = LinkedHashMap::new();
        for (index_minor, desc_minor) in event_desc.minors.iter().enumerate() {
            minor_map.insert(desc_minor.name.to_ascii_lowercase(), index_minor);
        }
        let name = if let Some(name) = event_desc.major.display_name {
            name
        } else {
            event_desc.major.name
        };
        map.insert(name.to_ascii_lowercase(), (index, minor_map));
    }
    map
});
