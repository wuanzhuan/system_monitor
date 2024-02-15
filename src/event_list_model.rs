use slint::{Model, ModelNotify, ModelTracker};
use std::{
    cell::RefCell, collections::{LinkedList, linked_list::CursorMut}, rc::Rc
};
use linked_hash_map::LinkedHashMap;


pub struct ListModel<'a: 'static, T> {
    // the backing data, access by cursor
    list: Box<LinkedList<Rc<T>>>,
    //reference the list in a `RefCell` as this model can be modified
    cursor: RefCell<CursorMut<'a, Rc<T>>>,
    // the ModelNotify will allow to notify the UI that the model changes
    notify: ModelNotify,
    pub stack_walk_map: RefCell<LinkedHashMap::<(u32, i64), Rc<T>>>
}

impl<'a, T: Clone + 'static> Model for ListModel<'a, T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.list.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row >= self.list.len() {
            return None;
        }
        self.move_to(row);
        let mut cursor = self.cursor.borrow_mut();
        cursor.current().map(|item| (**item).clone() )
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {
        if row >= self.list.len() {
            return;
        }
        self.move_to(row);
        let mut cursor = self.cursor.borrow_mut();
        if let Some(item) = cursor.current() {
            *item = Rc::new(data);
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
impl<'a, T> ListModel<'a, T> {
    pub fn new() -> Self {
        let p = Box::leak(Box::new(LinkedList::<Rc<T>>::default())) as *mut LinkedList<Rc<T>>; 
        let cursor = RefCell::new(unsafe{ &mut *p }.cursor_front_mut());
        let list = unsafe{ Box::from_raw(p) };
        let stack_walk_map = RefCell::new(LinkedHashMap::<(u32, i64), Rc<T>>::with_capacity(50));
        let list_model = Self { list, notify: Default::default(), cursor, stack_walk_map};
        list_model
    }

    fn move_to(&self, index: usize) {
        debug_assert!(index < self.list.len());
        let mut cursor = self.cursor.borrow_mut();
        let cursor_index =  if let Some(index) = cursor.index() {
            index
        } else {
            self.list.len()
        };
        if cursor_index == index {
            return;
        } else if cursor_index < index {
            if (index - cursor_index) * 2 <= self.list.len() {
                move_next_to_uncheck(&mut cursor, index);
            } else {
                move_prev_to_uncheck(&mut cursor, index);
            }
        } else {
            if (cursor_index - index) * 2 <= self.list.len() {
                move_prev_to_uncheck(&mut cursor, index);
            } else {
                move_next_to_uncheck(&mut cursor, index);
            }
        }
        return;

        fn move_next_to_uncheck<'a, T>(cursor: &mut CursorMut<'a, T>, index: usize) {
            loop {
                cursor.move_next();
                if let Some(i) = cursor.index() {
                    if index == i {
                        break;
                    }
                }
            };
        }
    
        fn move_prev_to_uncheck<'a, T>(cursor: &mut CursorMut<'a, T>, index: usize) {
            loop {
                cursor.move_prev();
                if let Some(i) = cursor.index() {
                    if index == i {
                        break;
                    }
                }
            };
        }
    }

    /// Add a row at the end of the model
    pub fn push(&self, value: Rc<T>) {
        let mut cursor = self.cursor.borrow_mut();
        cursor.push_back(value);
        self.notify.row_added(self.list.len() - 1, 1)
    }

    /// Remove the row at the given index from the model
    #[allow(unused)]
    pub fn remove(&self, index: usize) {
        if index >= self.list.len() {
            return;
        }
        let mut cursor = self.cursor.borrow_mut();
        self.move_to(index);
        cursor.remove_current();
        self.notify.row_removed(index, 1)
    }

    pub fn row_data_detail(&self, row: usize) -> Option<Rc<T>> {
        if row >= self.list.len() {
            return None;
        }
        self.move_to(row);
        let mut cursor = self.cursor.borrow_mut();
        cursor.current().map(|some| some.clone())
    }

}
