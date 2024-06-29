use crate::{event_list_model::ListModel, App, EventsViewData};
use parking_lot::FairMutex;
use slint::{ComponentHandle, Model, Weak};
use smol::{Task, Timer};
use std::time::Duration;

pub enum Notify {
    Push(/*index*/ usize, /*count*/ usize),
    Remove,
}

pub struct DelayNotify {
    status: FairMutex<DelayNotifyStatus>,
    max_count: usize,
    interval_ms: u64,
    timer_task: Option<Task<()>>,
}

struct DelayNotifyStatus {
    push_index: usize,
    push_count: usize,
    is_notified: bool,
    is_removed: bool,
}

impl DelayNotify {
    pub fn new(max_count: usize, interval_ms: u64) -> Self {
        DelayNotify {
            status: FairMutex::new(DelayNotifyStatus {
                push_index: 0,
                push_count: 0,
                is_notified: false,
                is_removed: false,
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
                    let mut lock = self_context.status.lock();
                    if lock.is_removed {
                        lock.push_index = 0;
                        lock.push_count = 0;
                        lock.is_notified = true;
                        lock.is_removed = false;
                        Self::notify_to_app(app_weak.clone(), Notify::Remove);
                    } else {
                        if !lock.is_notified {
                            let notify = Notify::Push(lock.push_index, lock.push_count);
                            lock.push_count = 0;
                            lock.is_notified = true;
                            Self::notify_to_app(app_weak.clone(), notify);
                        } else {
                            lock.is_notified = false;
                        }
                    }
                }
                Timer::after(period).await;
            }
        }));
    }

    pub fn notify(&self, app_weak: Weak<App>, notify: Notify) {
        let mut status = self.status.lock();
        // merge notify
        match notify {
            Notify::Push(index, count) => {
                assert!(count == 1);
                // when no item for waiting notify
                if status.push_count == 0 {
                    status.push_index = index;
                    status.push_count = count;
                } else {
                    // consider when removed but not notified
                    if index == status.push_index + status.push_count {
                        status.push_count += count;
                    } else {
                        // wait remove notify
                    }
                }
                if status.push_count >= self.max_count && !status.is_removed {
                    let notify = Notify::Push(status.push_index, status.push_count);
                    status.push_count = 0;
                    status.is_notified = true;
                    Self::notify_to_app(app_weak, notify);
                }
            }
            Notify::Remove => {
                status.is_removed = true;
            }
        }
    }

    fn notify_to_app(app_weak: Weak<App>, notify: Notify) {
        app_weak
            .upgrade_in_event_loop(move |app_handle| {
                let row_data = app_handle.global::<EventsViewData>().get_row_data();
                let rows = row_data.as_any().downcast_ref::<ListModel>().unwrap();
                match notify {
                    Notify::Push(index, count) => rows.notify_push(index, count),
                    Notify::Remove => rows.notify_reset(),
                }
            })
            .unwrap();
    }
}
