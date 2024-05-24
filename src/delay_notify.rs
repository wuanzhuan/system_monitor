use crate::event_list_model::ListModel;
use crate::{App, EventsViewData};
use parking_lot::{FairMutex, FairMutexGuard};
use slint::{ComponentHandle, Model, Weak};
use smol::{Task, Timer};
use std::time::Duration;

#[derive(PartialEq)]
pub enum NotifyType {
    Push,
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
                        Self::notify_to_app(data, app_weak.clone(), NotifyType::Push, index, count);
                    } else {
                        data.is_notified = false;
                    }
                }
                Timer::after(period).await;
            }
        }));
    }

    pub fn notify(&self, app_weak: Weak<App>, index: usize, notify_type: NotifyType) {
        let mut data = self.data.lock();
        // merge notify
        if notify_type == NotifyType::Push {
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
                Self::notify_to_app(data, app_weak, NotifyType::Push, index, count);
            }
        } else if notify_type == NotifyType::Remove {
            if data.count == 0 {
                data.is_notified = true;
                Self::notify_to_app(data, app_weak, NotifyType::Remove, index, 1);
            } else if index < data.index {
                data.index -= 1;
                data.is_notified = true;
                Self::notify_to_app(data, app_weak, NotifyType::Remove, index, 1);
            } else {
                data.count -= 1;
            }
        }
    }

    fn notify_to_app(
        mut data: FairMutexGuard<'_, NotifyPush>,
        app_weak: Weak<App>,
        ty: NotifyType,
        index: usize,
        count: usize,
    ) {
        data.index = index;
        data.count = 0;
        drop(data);
        app_weak
            .upgrade_in_event_loop(move |app_handle| {
                let row_data = app_handle.global::<EventsViewData>().get_row_data();
                let rows = row_data.as_any().downcast_ref::<ListModel>().unwrap();
                if ty == NotifyType::Push {
                    rows.notify_push(index, count);
                } else {
                    rows.notify_remove(index, count);
                }
            })
            .unwrap();
    }
}
