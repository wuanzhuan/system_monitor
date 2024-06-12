use crate::{event_list_model::ListModel, App, EventsViewData};
use parking_lot::{FairMutex, FairMutexGuard};
use slint::{ComponentHandle, Model, Weak};
use smol::{Task, Timer};
use std::time::Duration;

pub enum Notify {
    Push(/*index*/ usize, /*count*/ usize),
    Remove,
}

pub struct DelayNotify {
    data: FairMutex<NotifyPush>,
    max_count: usize,
    interval_ms: u64,
    timer_task: Option<Task<()>>,
}

struct NotifyPush {
    index: usize,
    count: usize,
    is_notified: bool,
}

impl DelayNotify {
    pub fn new(max_count: usize, interval_ms: u64) -> Self {
        DelayNotify {
            data: FairMutex::new(NotifyPush {
                index: 0,
                count: 0,
                is_notified: false,
            }),
            max_count,
            timer_task: None,
            interval_ms,
        }
    }

    pub fn init(&mut self, app_weak: Weak<App>) {
        let self_context = unsafe { &*(self as *const DelayNotify) };
        self.timer_task = Some(smol::spawn(async move {
            let period = Duration::from_millis(self_context.interval_ms);
            let app_weak = app_weak.clone();
            loop {
                {
                    let mut data = self_context.data.lock();
                    if !data.is_notified {
                        let index = data.index;
                        let count = data.count;
                        Self::notify_to_app(data, app_weak.clone(), Notify::Push(index, count));
                    } else {
                        data.is_notified = false;
                    }
                }
                Timer::after(period).await;
            }
        }));
    }

    pub fn notify(&self, app_weak: Weak<App>, index: usize, notify: Notify) {
        let mut data = self.data.lock();
        // merge notify
        match notify {
            Notify::Push(index, count) => {
                // data.count == 0 if no item for waiting notify
                if data.count == 0 {
                    data.index = index;
                    data.count = 1;
                } else {
                    assert!(index == data.index + data.count);
                    data.count += 1;
                }
                if data.count >= self.max_count {
                    let index = data.index;
                    let count = data.count;
                    data.is_notified = true;
                    Self::notify_to_app(data, app_weak, Notify::Push(index, count));
                }
            }
            Notify::Remove => {
                if data.count == 0 {
                    data.is_notified = true;
                    Self::notify_to_app(data, app_weak, Notify::Remove);
                } else if index < data.index {
                    data.index -= 1;
                    data.is_notified = true;
                    Self::notify_to_app(data, app_weak, Notify::Remove);
                } else {
                    data.count -= 1;
                }
            }
        }
    }

    fn notify_to_app(
        mut data: FairMutexGuard<'_, NotifyPush>,
        app_weak: Weak<App>,
        notify: Notify,
    ) {
        if let Notify::Push(index, count) = notify {
            data.index = index;
            data.count = 0;
            drop(data);
        }
        app_weak
            .upgrade_in_event_loop(move |app_handle| {
                let row_data = app_handle.global::<EventsViewData>().get_row_data();
                let rows = row_data.as_any().downcast_ref::<ListModel>().unwrap();
                match notify {
                    Notify::Push(index, count) => rows.notify_push(index, count),
                    Notify::Remove => rows.notify_remove(),
                }
            })
            .unwrap();
    }
}
