use slint::{Model, SharedString, ModelNotify, ModelTracker, StandardListViewItem};
use super::event_trace::EventRecordDecoded;

pub struct EventRecordModel{
    array: Box<EventRecordDecoded>,
    notify: ModelNotify
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
            array: Box::new(event_record),
            notify: ModelNotify::default()
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
                0 => Some(StandardListViewItem::from(SharedString::from(self.array.dt_local.to_string()))),
                1 => Some(StandardListViewItem::from(SharedString::from(self.array.process_id.to_string()))),
                2 => Some(StandardListViewItem::from(SharedString::from(self.array.thread_id.to_string()))),
                3 => Some(StandardListViewItem::from(SharedString::from(self.array.event_name.to_string()))),
                4 => Some(StandardListViewItem::from(SharedString::from(self.array.opcode_name.to_string()))),
                5 => Some(StandardListViewItem::from(SharedString::from(serde_json::to_string(&self.array.properties).unwrap_or_default()))),
                _ => None
            }
        }
    }

    fn set_row_data(&self, row: usize, data: Self::Data) { 
        // if set don't forget to call row_changed
        //self.notify.row_changed(row);
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn core::any::Any {
        // a typical implementation just return `self`
        self
    }
}