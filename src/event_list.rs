use anyhow::Result;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::linked_list::CursorMut;
use intrusive_collections::{linked_list::Cursor, LinkedList, LinkedListLink};
use parking_lot::FairMutex;
use std::{
    ptr,
    cell::SyncUnsafeCell,
    sync::{
        atomic::{AtomicUsize, AtomicU64, Ordering},
        Arc,
    },
};
use once_cell::sync::OnceCell;

#[derive(Clone)]
pub struct Node<T: Clone + Send + Sync> {
    link: LinkedListLink,
    pub serial_number: OnceCell<u64>,
    pub value: T,
}
unsafe impl<T: Clone + Send + Sync> Send for Node<T> {}
unsafe impl<T: Clone + Send + Sync> Sync for Node<T> {}

impl<T: Clone + Send + Sync> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            link: LinkedListLink::new(),
            serial_number: OnceCell::new(),
            value,
        }
    }
}

intrusive_adapter!(NodeAdapter<T> = Arc<Node<T>>: Node<T> { link: LinkedListLink } where T: Clone + Send + Sync);

struct NodeArc<T: Clone + Send + Sync> {
    node: Option<Arc<Node<T>>>,
    index: usize,
}

impl<T: Clone + Send + Sync> NodeArc<T> {
    fn new() -> Self {
        Self {
            node: None,
            index: 0,
        }
    }

    fn get_cursor<'a>(&self, list: &'a LinkedList<NodeAdapter<T>>) -> Option<Cursor<'a, NodeAdapter<T>>> {
        if let Some(ref node) = self.node {
            let cursor = unsafe{ list.cursor_from_ptr(node.as_ref()) };
            Some(cursor)
        } else {
            None
        }
    }

    fn set(&mut self, node: Option<Arc<Node<T>>>, index: usize) {
        self.index = if node.is_some() { index } else { 0 };
        self.node = node;
    }
}

pub struct EventList<T: Clone + Send + Sync> {
    // the backing data, access by cursor
    list: Box<SyncUnsafeCell<LinkedList<NodeAdapter<T>>>>,
    list_len: AtomicUsize,
    serial_number: AtomicU64, //todo: integer overflow
    list_except_last_lock: FairMutex<NodeArc<T>>,
    list_last_lock: FairMutex<()>,
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<T: Clone + Send + Sync> EventList<T> {
    pub fn new() -> Self {
        let list = Box::new(SyncUnsafeCell::new(LinkedList::<NodeAdapter<T>>::default()));
        let list_len = AtomicUsize::new(0);
        let serial_number = AtomicU64::new(0);
        let list_except_last_lock = FairMutex::new(NodeArc::new());
        let list_last_lock = FairMutex::new(());
        Self {
            list,
            list_len,
            serial_number,
            list_except_last_lock,
            list_last_lock,
        }
    }

    pub fn len(&self) -> usize {
        self.list_len.load(Ordering::Acquire)
    }

    pub fn get_by_index(&self, index_to: usize) -> Option<Arc<Node<T>>> {
        let mut list_except_last_lock = self.list_except_last_lock.lock();
        let mut list_len = self.list_len.load(Ordering::Acquire);
        if index_to >= list_len {
            return None;
        }
        
        // i.e. the cursor is null at start
        let (mut cursor, cursor_index) = if let Some(cursor) = list_except_last_lock.get_cursor(self.get_list()) {
            (cursor, list_except_last_lock.index)
        } else {
            let cursor = self.get_list().front();
            if cursor.is_null() {
                return None;
            }
            (cursor, 0usize)
        };

        if cursor_index == index_to {
            list_except_last_lock.set(cursor.clone_pointer(), index_to);
            return cursor.clone_pointer();
        } else {
            if cursor_index < index_to {
                if index_to <= (list_len + cursor_index) / 2 {
                    move_next_to_uncheck(&mut cursor, cursor_index, index_to);
                } else {
                    let _list_last_lock = self.list_last_lock.lock();
                    list_len = self.list_len.load(Ordering::Acquire);
                    cursor = self.get_list().back();
                    drop(_list_last_lock);
                    move_prev_to_uncheck(&mut cursor, list_len - 1, index_to);
                }
            } else {
                if index_to >= cursor_index / 2 {
                    move_prev_to_uncheck(&mut cursor, cursor_index, index_to);
                } else {
                    let _list_last_lock = self.list_last_lock.lock();
                    cursor = self.get_list().front();
                    drop(_list_last_lock);
                    move_next_to_uncheck(&mut cursor, 0, index_to);
                }
            }
            list_except_last_lock.set(cursor.clone_pointer(), index_to);
            return cursor.clone_pointer();
        }

        // the function should be success
        fn move_next_to_uncheck<'a, T: Clone + Send + Sync> (
            cursor: &mut Cursor<'a, NodeAdapter<T>>,
            current_index: usize,
            index_to: usize,
        ) {
            let mut index = current_index;
            while index != index_to {
                assert!(!cursor.is_null(), "index: {index} index_to: {index_to}");
                cursor.move_next();
                index += 1;
            }
        }

        // the function should be success
        fn move_prev_to_uncheck<'a, T: Clone + Send + Sync>(
            cursor: &mut Cursor<'a, NodeAdapter<T>>,
            current_index: usize,
            index_to: usize,
        ) {
            let mut index = current_index;
            while index != index_to {
                assert!(!cursor.is_null(), "index: {index} index_to: {index_to}");
                cursor.move_prev();
                index -= 1;
            }
        }
    }

    pub fn traversal(&self, cb: impl Fn(&T) -> Result<bool>) -> Result<Vec<i32>> {
        let mut _list_except_last_lock = self.list_except_last_lock.lock();
        let list_len = self.list_len.load(Ordering::Acquire);
        let list = unsafe { &*self.list.get() };
        let mut vec = vec![];
        for (index, item) in list.iter().enumerate() {
            let is_find = cb(&item.value)?;
            if is_find {
                vec.push(index as i32);
            }
            if index as usize >= list_len {
                break;
            }
        }
        Ok(vec)
    }

    pub fn push(&self, value: Arc<Node<T>>) -> usize {
        let mut _list_last_lock = self.list_last_lock.lock();
        let serail_number = self.serial_number.fetch_add(1, Ordering::Release);
        value.serial_number.set(serail_number).unwrap();
        unsafe { &mut *self.list.get() }.push_back(value);
        let index = self.list_len.fetch_add(1, Ordering::Release);
        index
    }

    /// Remove the row by a arc
    pub fn remove(&self, node_arc: Arc<Node<T>>) {
        let mut list_except_last_lock = self.list_except_last_lock.lock(); // lock before remove
        let mut cursor = self.get_cursor_mut_from_node(node_arc.as_ref());  // the node_arc may be modify by other 
        // the remove and push is called in same thread
        let node_arc_removed = cursor.remove();
        if let Some(node_arc_removed) = &node_arc_removed {
            self.list_len.fetch_sub(1, Ordering::Release);
            if let Some(cursor_last_read) = &list_except_last_lock.node {
                if ptr::eq(node_arc_removed.as_ref(), cursor_last_read.as_ref()) {
                    let index = list_except_last_lock.index;
                    list_except_last_lock.set(cursor.as_cursor().clone_pointer(), index);
                } else {
                    if node_arc_removed.serial_number.get().unwrap() <= cursor_last_read.serial_number.get().unwrap() {
                        if list_except_last_lock.index != 0 {
                            list_except_last_lock.index -= 1;
                        }
                    }
                }
            }
        }
    }

    fn get_list(&self) -> &LinkedList<NodeAdapter<T>> {
        unsafe{ &*self.list.get() }
    }

    fn get_list_mut(&self) -> &mut LinkedList<NodeAdapter<T>> {
        unsafe{ &mut *self.list.get() }
    }

    fn get_cursor_mut_from_node(&self, node: &Node<T>) -> CursorMut<NodeAdapter<T>> {
        unsafe { self.get_list_mut().cursor_mut_from_ptr(node) }
    }

}
