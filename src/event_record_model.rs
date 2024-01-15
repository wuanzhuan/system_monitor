use slint::{Model, SharedString, ModelNotify, ModelTracker, StandardListViewItem};
use super::event_trace::EventRecordDecoded;

#[derive(Default)]
pub struct EventRecordModel{
    array: Vec<SharedString>,
    notify: ModelNotify
}

impl EventRecordModel {
    pub fn new(event_record: &EventRecordDecoded) -> Self {
        EventRecordModel{
            array: vec![
                SharedString::from(event_record.dt_local.to_string()),
                SharedString::from(event_record.process_id.to_string()),
                SharedString::from(event_record.thread_id.to_string()),
                SharedString::from(event_record.event_name.clone()),
                SharedString::from(event_record.opcode_name.clone()),
                SharedString::from(serde_json::to_string(&event_record.properties).unwrap_or_default()),
            ],
            notify: ModelNotify::default()
        }
    }
}

impl Model for EventRecordModel {
    type Data = StandardListViewItem;

    fn row_count(&self) -> usize {
        self.array.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row < self.array.len() {
            Some(StandardListViewItem::from(self.array[row].clone()))
        } else {
            None
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
