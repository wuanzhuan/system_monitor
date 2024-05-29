use crate::event_list::EventList;
use crate::event_list::Node;
use crate::event_record_model::EventRecordModel;
use crate::filter_expr::{FilterExpr, evaluate};
use anyhow::Result;
use slint::{Model, ModelNotify, ModelRc, ModelTracker, StandardListViewItem};
use std::sync::Arc;

pub struct ListModel<'a: 'static> {
    // the backing data, access by cursor
    list: Arc<EventList<'a, EventRecordModel>>,
    // the ModelNotify will allow to notify the UI that the model changes
    notify: ModelNotify,
}

impl<'a> Model for ListModel<'a> {
    type Data = ModelRc<StandardListViewItem>;

    fn row_count(&self) -> usize {
        self.list.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row >= self.list.len() {
            return None;
        }
        self.list
            .get_by_index(row)
            .map(|some| ModelRc::new(some.value.clone()))
    }

    fn set_row_data(&self, #[allow(unused)] row: usize, #[allow(unused)] data: Self::Data) {}

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn core::any::Any {
        // a typical implementation just return `self`
        self
    }
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<'a> ListModel<'a> {
    pub fn new(list: Arc<EventList<EventRecordModel>>) -> Self {
        Self {
            list,
            notify: Default::default(),
        }
    }

    /// Add a row at the end of the model
    pub fn notify_push(&self, index: usize, count: usize) {
        self.notify.row_added(index, count);
    }

    /// Remove the row at the given index from the model
    #[allow(unused)]
    pub fn notify_remove(&self, index: usize, count: usize) {
        self.notify.row_removed(index, count);
    }

    pub fn row_data_detail(&self, row: usize) -> Option<Arc<Node<EventRecordModel>>> {
        if row >= self.list.len() {
            return None;
        }
        self.list.get_by_index(row)
    }

    pub fn row_find(&self, filter_expr: &FilterExpr) -> Result<Vec<i32>> {
        self.list.traversal(|item| {
            evaluate(filter_expr, |path, value| {
                item.find_by_path_value(path, value)
            }, |value| {
                item.find_by_value(value)
            })
        })
    }
}
