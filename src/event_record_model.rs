use slint::{Model, ModelRc, ModelTracker, SharedString, StandardListViewItem, VecModel};
use super::event_trace::{EventRecordDecoded, StackWalk};
use crate::StackWalkInfo;
use std::sync::{OnceLock, Arc};


#[derive(Clone)]
pub struct EventRecordModel{
    array: Arc<EventRecordDecoded>,
    stack_walk: OnceLock<Arc<StackWalk>>,
}

const COLUMN_NAMES: &[&str] = &[
    "datetime",
    "process_id",
    "thread_id",
    "event_name",
    "opcode_name",
    "properties",
];

impl EventRecordModel {
    pub fn new(event_record: EventRecordDecoded) -> Self {
        EventRecordModel{
            array: Arc::new(event_record),
            stack_walk: OnceLock::new()
        }
    }

    pub fn data_detail(&self) -> Option<SharedString> {
        Some(SharedString::from(serde_json::to_string_pretty(&*self.array).unwrap_or_default()))
    }

    /// Returns true if the `stack_walk` is None
    pub fn set_stack_walk(&self, sw: StackWalk) -> bool {
        let ret = if self.stack_walk.get().is_some() { false } else { true };
        let _ = self.stack_walk.set(Arc::new(sw));
        ret
    }

    pub fn stack_walk(&self) -> StackWalkInfo {
        if let Some(sw) = self.stack_walk.get() {
            let vec = VecModel::<SharedString>::default();
            for item in sw.stacks.iter() {
                let str = format!("{}: {:#x}", item.0, item.1);
                vec.push(SharedString::from(str.as_str()))
            }
            StackWalkInfo{
                event_timestamp: SharedString::from(sw.event_timestamp.to_string()), 
                process_id: SharedString::from(format!("{}", sw.stack_process as i32)), 
                thread_id: SharedString::from(format!("{}", sw.stack_thread as i32)),
                stacks: ModelRc::<SharedString>::new(vec)}
        } else {
            StackWalkInfo::default()
        }
    }

}

impl Model for EventRecordModel {
    type Data = StandardListViewItem;

    fn row_count(&self) -> usize {
        COLUMN_NAMES.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row >= COLUMN_NAMES.len() {
            None
        } else {
            match row {
                0 => Some(StandardListViewItem::from(SharedString::from(self.array.timestamp.to_datetime_detail()))),
                1 => Some(StandardListViewItem::from(SharedString::from((self.array.process_id as i32).to_string()))),
                2 => Some(StandardListViewItem::from(SharedString::from((self.array.thread_id as i32).to_string()))),
                3 => Some(StandardListViewItem::from(SharedString::from(self.array.event_name.to_string()))),
                4 => Some(StandardListViewItem::from(SharedString::from(self.array.opcode_name.to_string()))),
                5 => Some(StandardListViewItem::from(SharedString::from(serde_json::to_string(&self.array.properties).unwrap_or_default()))),
                _ => None
            }
        }
    }

    fn set_row_data(&self, #[allow(unused)] row: usize, #[allow(unused)] data: Self::Data) { 
        // if set don't forget to call row_changed
        //self.notify.row_changed(row);
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn core::any::Any {
        // a typical implementation just return `self`
        self
    }
}
