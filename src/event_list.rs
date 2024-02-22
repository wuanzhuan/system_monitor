use std::{
    cell::SyncUnsafeCell, sync::{
        atomic::{
            AtomicUsize, Ordering
        }, Arc, RwLock, RwLockWriteGuard
    }
};
use linked_hash_map::LinkedHashMap;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{LinkedList, LinkedListLink, linked_list::Cursor};


pub struct Node<T> {
    link: LinkedListLink,
    pub value: T,
}
unsafe impl<T> Send for Node<T> {}
unsafe impl<T> Sync for Node<T> {}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            link: LinkedListLink::new(),
            value
        }
    }
}

intrusive_adapter!(NodeAdapter<T> = Arc<Node<T>>: Node<T> { link: LinkedListLink });

struct CursorSync<'a, T>(pub Cursor<'a, NodeAdapter<T>>);
unsafe impl<'a, T> Send for CursorSync<'a, T> {}
unsafe impl<'a, T> Sync for CursorSync<'a, T> {}

pub struct EventList<'a: 'static, T> {
    // the backing data, access by cursor
    list: SyncUnsafeCell<Box<LinkedList<NodeAdapter<T>>>>,
    list_len: AtomicUsize,
    reader_lock: RwLock<(CursorSync<'a, T>, usize)>, /// only protect the cursor_reader
    push_back_lock: RwLock<()>, /// protect the tail node and the cursor_push_back

    // the ModelNotify will allow to notify the UI that the model changes
    pub stack_walk_map: SyncUnsafeCell<LinkedHashMap::<(u32, i64), Arc<Node<T>>>>
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<'a, T> EventList<'a, T> {
    pub fn new() -> Self {
        let list = SyncUnsafeCell::new(Box::new(LinkedList::<NodeAdapter<T>>::default()));
        let list_len = AtomicUsize::new(0);
        let reader_lock = RwLock::new((CursorSync(unsafe{ &mut *list.get() }.cursor()), 0));
        let push_back_lock = RwLock::new(());
        let stack_walk_map = SyncUnsafeCell::new(LinkedHashMap::<(u32, i64), Arc<Node<T>>>::with_capacity(50));
        Self { list, list_len, reader_lock, push_back_lock, stack_walk_map }
    }

    pub fn len(&self) -> usize {
        self.list_len.load(Ordering::Acquire)
    }

    pub fn get_by_index(&self, index_to: usize) -> Option<Arc<Node<T>>>{
        let mut reader_guard = self.reader_lock.write().unwrap();

        let list_len = self.list_len.load(Ordering::Acquire);
        if index_to >= list_len {
            return None;
        }

        let cursor_index =  if !reader_guard.0.0.is_null() {
            reader_guard.1
        } else {
            list_len
        };

        if cursor_index == index_to {
            return reader_guard.0.0.clone_pointer();
        } else if cursor_index < index_to {
            if (index_to - cursor_index) * 2 <= list_len {
                move_next_to_uncheck(&mut reader_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.reader_lock.read().unwrap();
                let list_len = self.list_len.load(Ordering::Acquire);
                reader_guard.0 = CursorSync(unsafe{&*self.list.get()}.front());
                reader_guard.1 = 0;
                move_prev_to_uncheck(&mut reader_guard, index_to, list_len);
            }
        } else {
            if (cursor_index - index_to) * 2 <= list_len {
                move_prev_to_uncheck(&mut reader_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.reader_lock.read().unwrap();
                let list_len = self.list_len.load(Ordering::Acquire);
                reader_guard.0 = CursorSync(unsafe{&*self.list.get()}.back());
                reader_guard.1 = list_len - 1;
                move_next_to_uncheck(&mut reader_guard, index_to, list_len);
            }
        }
        return reader_guard.0.0.clone_pointer();

        fn move_next_to_uncheck<'a, T>(reader_guard: &mut RwLockWriteGuard<'_, (CursorSync<'_, T>, usize)>, index_to: usize, list_len: usize) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = reader_guard.0.0.is_null();
                reader_guard.0.0.move_next();
                if reader_guard.0.0.is_null() {
                    if prev_is_null {
                        reader_guard.1 = 0;
                        break;
                    }
                    reader_guard.1 = list_len;
                } else {
                    if prev_is_null {
                        reader_guard.1 = 0;
                    } else {
                        reader_guard.1 += 1;
                    }
                    if reader_guard.1 == index_to {
                        break;
                    }
                }
            };
        }
    
        fn move_prev_to_uncheck<'a, T>(reader_guard: &mut RwLockWriteGuard<'_, (CursorSync<'_, T>, usize)>, index_to: usize, list_len: usize) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = reader_guard.0.0.is_null();
                reader_guard.0.0.move_prev();
                if reader_guard.0.0.is_null() {
                    if prev_is_null {
                        reader_guard.1 = 0;
                        break;
                    }
                    reader_guard.1 = list_len;
                } else {
                    if prev_is_null {
                        reader_guard.1 = list_len - 1;
                    } else {
                        reader_guard.1 -= 1;
                    }
                    if reader_guard.1 == index_to {
                        break;
                    }
                }
            };
        }
    }

    pub fn push(&self, value: Arc<Node<T>>) -> usize {
        let mut _push_back_guard = self.push_back_lock.write().unwrap();
        unsafe{ &mut *self.list.get() }.push_back(value);
        let index = self.list_len.fetch_add(1, Ordering::Release);
        drop(_push_back_guard);
        index
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

    pub fn get_stack_walk_map_mut(&self) -> &mut LinkedHashMap::<(u32, i64), Arc<Node<T>>> {
        unsafe{
            let map = &mut *self.stack_walk_map.get();
            map
        }
    }

}
