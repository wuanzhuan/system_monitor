use slint::{Model, ModelNotify, ModelTracker};
use std::{
    cell::RefCell,
    collections::LinkedList
};

#[derive(Default)]
pub struct ListModel<T> {
    // the backing data, stored in a `RefCell` as this model can be modified
    list: RefCell<LinkedList<T>>,
    // the ModelNotify will allow to notify the UI that the model changes
    notify: ModelNotify,
}

impl<T: Clone + 'static> Model for ListModel<T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.list.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.list.borrow().iter().nth(row).cloned()
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {
        if let Some(item) = self.list.borrow_mut().iter_mut().nth(row) {
            *item = data;
            // don't forget to call row_changed
            self.notify.row_changed(row);
        }
    }

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
impl<T> ListModel<T> {
    /// Add a row at the end of the model
    pub fn push(&self, value: T) {
        self.list.borrow_mut().push_back(value);
        self.notify.row_added(self.list.borrow().len() - 1, 1)
    }

    /// Remove the row at the given index from the model
    pub fn remove(&self, index: usize) {
        //self.list.borrow_mut().remove(index);
        self.notify.row_removed(index, 1)
    }
}
