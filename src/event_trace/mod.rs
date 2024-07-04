use crate::third_extend::strings::*;
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
use tracing::{error, warn};
use widestring::*;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{Diagnostics::Etw::*, SystemInformation::*},
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
                match r {
                    Ok(_) => {
                        context_mg.h_trace_session = h_trace;
                        break Ok(());
                    }
                    Err(e) => {
                        if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                            if let Err(e) = unsafe {
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
                            warn!(
                                "The {session_name:#?} is already exist. and stop before restart"
                            );
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
                    EventRecordCallback: Some(Controller::callback),
                },
                ..Default::default()
            };
            let h_consumer = unsafe { OpenTraceW(&mut trace_log) };
            if INVALID_PROCESSTRACE_HANDLE == h_consumer.Value {
                context_mg.event_record_callback = None;
                break Err(anyhow!("Failed to OpenTraceW: {:#?}", Error::from_win32()));
            }
            context_mg.h_trace_consumer = h_consumer;
            drop(context_mg);

            let (tx, rx) = mpsc::channel::<ErrorAnyhow>();
            let h_thread = thread::spawn(move || {
                let ft_now = unsafe { GetSystemTimeAsFileTime() };
                let r = match unsafe { ProcessTrace(&[h_consumer], Some(&ft_now), None) } {
                    Err(e) => {
                        let r_send = tx.send(anyhow!("Failed to ProcessTrace: {}", e));
                        if r_send.is_ok() {
                            return;
                        }
                        Err(e.into())
                    }
                    Ok(_) => Ok(()),
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
            if let Err(e) = error {
                error!("failed to ControlTraceW {}", e);
            }
        }

        if INVALID_PROCESSTRACE_HANDLE != context_mg.h_trace_consumer.Value {
            let h_trace_consumer = context_mg.h_trace_consumer;
            drop(context_mg);
            let error = unsafe { CloseTrace(h_trace_consumer) };
            context_mg = CONTEXT.lock();
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
            context_mg = CONTEXT.lock();
        }

        // clear other
        let _ = context_mg.event_record_callback.take();
        context_mg.unstored_events_map.borrow_mut().clear();

        Ok(())
    }

    pub fn set_config_enables(index_major: usize, index_minor: Option<usize>, checked: bool) {
        let mut context_mg = CONTEXT.lock();
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
    }

    unsafe extern "system" fn callback(eventrecord: *mut EVENT_RECORD) {
        let er: &EVENT_RECORD = mem::transmute(eventrecord);
        let is_stack_walk = er.EventHeader.ProviderId == event_kernel::STACK_WALK_GUID;
        let is_module_event =
            er.EventHeader.ProviderId == ImageLoadGuid || er.EventHeader.ProviderId == ProcessGuid;
        let is_lost_event = er.EventHeader.ProviderId == LOST_EVENT_GUID;
        let is_auto_generated = er.EventHeader.ProviderId == EventTraceGuid;

        let context_mg = CONTEXT.lock();
        if context_mg.is_stopping {
            return;
        }
        let event_indexes = match get_event_indexes(er, Some(&context_mg)) {
            Ok(indexes) => indexes,
            Err(e) => {
                error!("{e}");
                if !is_stack_walk {
                    context_mg
                    .unstored_events_map
                    .borrow_mut()
                    .insert((er.EventHeader.ThreadId, er.EventHeader.TimeStamp), (), format!("{:?}-{:?}", er.EventHeader.ProviderId, er.EventHeader.EventDescriptor.Opcode));
                }
                return;
            }
        };

        // filter non stack walk events
        let mut is_enabled = false;
        if !is_stack_walk {
            let event_enable = &context_mg.config.events_enables[event_indexes.0];
            if event_enable.major {
                if event_enable.minors[event_indexes.1] {
                    is_enabled = true;
                }
            } else {
                // the major event is filter by flag. so a error happens when a event that is not enable comes
                // the EventTrace Process Image event is always enable.
                if !is_module_event && !is_auto_generated {
                    error!(
                        "No enable major event is coming: {}-{} event_record: {}",
                        EVENTS_DESC[event_indexes.0].major.name,
                        EVENTS_DESC[event_indexes.0].minors[event_indexes.1].name,
                        EventRecord(er)
                    );
                }
            }
            if !is_enabled {
                // escape event
                if !is_module_event && !is_lost_event {
                    context_mg
                        .unstored_events_map
                        .borrow_mut()
                        .insert((er.EventHeader.ThreadId, er.EventHeader.TimeStamp), (), format!("{:?}-{:?}", er.EventHeader.ProviderId, er.EventHeader.EventDescriptor.Opcode));
                    return;
                }
            }
        };
        drop(context_mg);

        let mut event_record_decoded = match event_decoder::Decoder::new(er) {
            Ok(mut decoder) => match decoder.decode() {
                Ok(event_record_decoded) => event_record_decoded,
                Err(e) => {
                    warn!("Faild to decode: {e} EventRecord: {}", EventRecord(er));
                    event_decoder::decode_kernel_event(
                        er,
                        event_kernel::EVENTS_DESC[event_indexes.0].major.name,
                        event_kernel::EVENTS_DESC[event_indexes.0].minors[event_indexes.1].name,
                    )
                }
            },
            Err(e) => {
                warn!(
                    "Faild to Decoder::new: {e} EventRecord: {}",
                    EventRecord(er)
                );
                event_decoder::decode_kernel_event(
                    er,
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
            let sw = StackWalk::from_event_record_decoded(&event_record_decoded);
            if context_mg
                .unstored_events_map
                .borrow_mut()
                .remove(&(sw.stack_thread, sw.event_timestamp), er.EventHeader.TimeStamp)
                .is_none()
            {
                let cb = context_mg.event_record_callback.clone().unwrap();
                mem::drop(context_mg);
                let cb = unsafe { &mut *cb.get() };
                cb(event_record_decoded, Some(sw), false);
            } else {
                error!(
                    "Removed stack walk: {}:{}:{} for the unstored event: {}:{}:{}",
                    sw.stack_process,
                    sw.stack_thread as i32,
                    crate::utils::TimeStamp(sw.event_timestamp).to_string_detail(),
                    er.EventHeader.ProcessId as i32,
                    er.EventHeader.ThreadId as i32,
                    crate::utils::TimeStamp(er.EventHeader.TimeStamp).to_string_detail(),
                );
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
        if let Err(e) = unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceSystemTraceEnableFlagsInfo,
                ptr::addr_of!(gm.masks) as *const ffi::c_void,
                mem::size_of_val(&gm.masks) as u32,
            )
        } {
            error!(
                "Failed to TraceSetInformation TraceSystemTraceEnableFlagsInfo: {}",
                e
            );
            return Err(e.into());
        }
        let (vec_event_id, size) = self.config.get_classic_event_id_vec();
        if let Err(e) = unsafe {
            TraceSetInformation(
                self.h_trace_session,
                TraceStackTracingInfo,
                vec_event_id.as_ptr() as *const ffi::c_void,
                size as u32,
            )
        } {
            error!("Failed to TraceSetInformation TraceStackTracingInfo: {}", e);
            return Err(e.into());
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
    properties.Wnode.ClientContext = 2; // if 1 the StackWalk event's timestamp is invalid. because of not set PROCESS_TRACE_MODE_RAW_TIMESTAMP of EVENT_TRACE_LOGFILEA
    properties.BufferSize = 512 * 1024;
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
