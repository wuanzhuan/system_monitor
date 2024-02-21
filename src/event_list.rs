use std::{
    cell::SyncUnsafeCell,
    sync::{
        Arc, RwLock, RwLockWriteGuard,
        atomic::{
            AtomicUsize, Ordering
        }
    },
};
use linked_hash_map::LinkedHashMap;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{LinkedList, LinkedListLink, linked_list::{Cursor, CursorMut}};


pub struct Node<T> {
    link: LinkedListLink,
    pub value: T,
}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            link: LinkedListLink::new(),
            value
        }
    }
}

intrusive_adapter!(NodeAdapter<T> = Arc<Node<T>>: Node<T> { link: LinkedListLink });


pub struct EventList<'a: 'static, T> {
    // the backing data, access by cursor
    list: SyncUnsafeCell<Box<LinkedList<NodeAdapter<T>>>>,
    list_len: AtomicUsize,
    cursor_reader: RwLock<(Cursor<'a, NodeAdapter<T>>, usize)>, /// only protect the cursor_reader
    cursor_push_back: RwLock<CursorMut<'a, NodeAdapter<T>>>, /// protect the tail node and the cursor_push_back
    cursor_remove: SyncUnsafeCell<CursorMut<'a, NodeAdapter<T>>>,

    // the ModelNotify will allow to notify the UI that the model changes
    pub stack_walk_map: RwLock<LinkedHashMap::<(u32, i64), Arc<Node<T>>>>
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<'a, T> EventList<'a, T> {
    pub fn new() -> Self {
        let list = SyncUnsafeCell::new(Box::new(LinkedList::<NodeAdapter<T>>::default()));
        let list_len = AtomicUsize::new(0);
        let cursor_reader = RwLock::new((unsafe{ &mut *list.get() }.cursor(), 0));
        let cursor_push_back = RwLock::new(unsafe{ &mut *list.get() }.cursor_mut());
        let cursor_remove = SyncUnsafeCell::new(unsafe{ &mut *list.get() }.cursor_mut());
        let stack_walk_map = RwLock::new(LinkedHashMap::<(u32, i64), Arc<Node<T>>>::with_capacity(50));
        Self { list, list_len, cursor_reader, cursor_push_back, cursor_remove, stack_walk_map }
    }

    pub fn get_by_index(&self, index_to: usize) -> Option<Arc<Node<T>>>{
        let mut cursor_guard = self.cursor_reader.write().unwrap();

        let list_len = self.list_len.load(Ordering::Acquire);
        if index_to >= list_len {
            return None;
        }

        let cursor_index =  if !cursor_guard.0.is_null() {
            cursor_guard.1
        } else {
            list_len
        };

        if cursor_index == index_to {
            return None;
        } else if cursor_index < index_to {
            if (index_to - cursor_index) * 2 <= list_len {
                move_next_to_uncheck(&mut cursor_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.cursor_reader.read().unwrap();
                let list_len = self.list_len.load(Ordering::Acquire);
                cursor_guard.0 = unsafe{&*self.list.get()}.front();
                cursor_guard.1 = 0;
                move_prev_to_uncheck(&mut cursor_guard, index_to, list_len);
            }
        } else {
            if (cursor_index - index_to) * 2 <= list_len {
                move_prev_to_uncheck(&mut cursor_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.cursor_reader.read().unwrap();
                let list_len = self.list_len.load(Ordering::Acquire);
                cursor_guard.0 = unsafe{&*self.list.get()}.back();
                cursor_guard.1 = list_len - 1;
                move_next_to_uncheck(&mut cursor_guard, index_to, list_len);
            }
        }
        return cursor_guard.0.clone_pointer();

        fn move_next_to_uncheck<'a, T>(cursor_guard: &mut RwLockWriteGuard<'_, (Cursor<'_, NodeAdapter<T>>, usize)>, index_to: usize, list_len: usize) {
            loop {
                let prev_is_null = cursor_guard.0.is_null();
                cursor_guard.0.move_next();
                if !cursor_guard.0.is_null() {
                    if prev_is_null {
                        cursor_guard.1 = 0;
                    } else {
                        cursor_guard.1 += 1;
                    }
                    if cursor_guard.1 == index_to {
                        break;
                    }
                } else {
                    cursor_guard.1 = list_len;
                }
            };
        }
    
        fn move_prev_to_uncheck<'a, T>(cursor_guard: &mut RwLockWriteGuard<'_, (Cursor<'_, NodeAdapter<T>>, usize)>, index_to: usize, list_len: usize) {
            loop {
                let prev_is_null = cursor_guard.0.is_null();
                cursor_guard.0.move_prev();
                if !cursor_guard.0.is_null() {
                    if prev_is_null {
                        cursor_guard.1 = list_len - 1;
                    } else {
                        cursor_guard.1 -= 1;
                    }
                    if cursor_guard.1 == index_to {
                        break;
                    }
                } else {
                    cursor_guard.1 = list_len;
                }
            };
        }
    }

    pub fn push(&self, value: Arc<Node<T>>) {
        let mut _cursor_guard = self.cursor_push_back.write().unwrap();
        unsafe{ &mut *self.list.get() }.push_back(value);
        self.list_len.fetch_add(1, Ordering::Release);
        // notify update push_back_ptr
    }

    /// Remove the row at the given index from the model
    #[allow(unused)]
    pub fn remove(&self, node_arc: Arc<Node<T>>) {
        let mut cursor = unsafe{ (&mut *self.list.get()).cursor_mut_from_ptr(node_arc.as_ref()) };
        if cursor.remove().is_some() {
            self.list_len.fetch_sub(1, Ordering::Release);
        }
        //notify
    }

}
