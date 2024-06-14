use anyhow::Result;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{linked_list::Cursor, LinkedList, LinkedListLink};
use parking_lot::{FairMutex, RwLock, RwLockWriteGuard};
use std::{
    mem,
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

struct CursorSync<'a, T: Clone + Send + Sync> {
    pub inner: Cursor<'a, NodeAdapter<T>>,
    pub index: usize,
}
unsafe impl<'a, T: Clone + Send + Sync> Send for CursorSync<'a, T> {}
unsafe impl<'a, T: Clone + Send + Sync> Sync for CursorSync<'a, T> {}

pub struct EventList<'a: 'static, T: Clone + Send + Sync> {
    // the backing data, access by cursor
    list: SyncUnsafeCell<Box<LinkedList<NodeAdapter<T>>>>,
    list_len: AtomicUsize,
    serial_number: AtomicU64, //todo: integer overflow
    list_except_last_lock: RwLock</*cursor_last_read*/CursorSync<'a, T>>,
    list_last_lock: FairMutex<()>,
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<'a: 'static, T: Clone + Send + Sync> EventList<'a, T> {
    pub fn new() -> Self {
        let list = SyncUnsafeCell::new(Box::new(LinkedList::<NodeAdapter<T>>::default()));
        let list_len = AtomicUsize::new(0);
        let serial_number = AtomicU64::new(0);
        let list_except_last_lock = RwLock::new(CursorSync {
            inner: unsafe { &mut *list.get() }.cursor(),
            index: 0,
        });
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
        let mut list_except_last_lock = self.list_except_last_lock.write();

        let list_len = self.list_len.load(Ordering::Acquire);
        if index_to >= list_len {
            return None;
        }

        let cursor_index = if !list_except_last_lock.inner.is_null() {
            list_except_last_lock.index
        } else {
            list_len
        };

        if cursor_index == index_to {
            return list_except_last_lock.inner.clone_pointer();
        } else if cursor_index < index_to {
            if (index_to - cursor_index) * 2 <= list_len {
                move_next_to_uncheck(&mut list_except_last_lock, index_to, list_len);
            } else {
                let _list_last_lock = self.list_last_lock.lock();
                let list_len = self.list_len.load(Ordering::Acquire);
                *list_except_last_lock = CursorSync {
                    inner: unsafe { &*self.list.get() }.front(),
                    index: 0,
                };
                move_prev_to_uncheck(&mut list_except_last_lock, index_to, list_len);
            }
        } else {
            if (cursor_index - index_to) * 2 <= list_len {
                move_prev_to_uncheck(&mut list_except_last_lock, index_to, list_len);
            } else {
                let _list_last_lock = self.list_last_lock.lock();
                let list_len = self.list_len.load(Ordering::Acquire);
                *list_except_last_lock = CursorSync {
                    inner: unsafe { &*self.list.get() }.back(),
                    index: list_len - 1,
                };
                move_next_to_uncheck(&mut list_except_last_lock, index_to, list_len);
            }
        }
        return list_except_last_lock.inner.clone_pointer();

        fn move_next_to_uncheck<'a, T: Clone + Send + Sync>(
            list_except_last_lock: &mut RwLockWriteGuard<'_, CursorSync<'_, T>>,
            index_to: usize,
            list_len: usize,
        ) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = list_except_last_lock.inner.is_null();
                list_except_last_lock.inner.move_next();
                if list_except_last_lock.inner.is_null() {
                    if prev_is_null {
                        list_except_last_lock.index = 0;
                        break;
                    }
                    list_except_last_lock.index = list_len;
                } else {
                    if prev_is_null {
                        list_except_last_lock.index = 0;
                    } else {
                        list_except_last_lock.index += 1;
                    }
                    if list_except_last_lock.index == index_to {
                        break;
                    }
                }
            }
        }

        fn move_prev_to_uncheck<'a, T: Clone + Send + Sync>(
            list_except_last_lock: &mut RwLockWriteGuard<'_, CursorSync<'_, T>>,
            index_to: usize,
            list_len: usize,
        ) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = list_except_last_lock.inner.is_null();
                list_except_last_lock.inner.move_prev();
                if list_except_last_lock.inner.is_null() {
                    if prev_is_null {
                        list_except_last_lock.index = 0;
                        break;
                    }
                    list_except_last_lock.index = list_len;
                } else {
                    if prev_is_null {
                        list_except_last_lock.index = list_len - 1;
                    } else {
                        list_except_last_lock.index -= 1;
                    }
                    if list_except_last_lock.index == index_to {
                        break;
                    }
                }
            }
        }
    }

    pub fn traversal(&self, cb: impl Fn(&T) -> Result<bool>) -> Result<Vec<i32>> {
        let mut _list_except_last_lock = self.list_except_last_lock.read();
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
        let mut cursor = unsafe { (&mut *self.list.get()).cursor_mut_from_ptr(node_arc.as_ref()) };
        let mut list_except_last_lock = self.list_except_last_lock.write(); // lock before remove
        if let Some(node_arc_removed) = cursor.remove() {
            self.list_len.fetch_sub(1, Ordering::Release);
            if let Some(cursor_last_read) = list_except_last_lock.inner.get() {
                if &*node_arc_removed as *const _ as *const () == cursor_last_read as *const _ as *const () {
                    *list_except_last_lock = CursorSync {
                        inner: unsafe{ mem::transmute(cursor.as_cursor()) },
                        index: list_except_last_lock.index - 1
                    };
                } else {
                    if node_arc_removed.serial_number.get().unwrap() <= cursor_last_read.serial_number.get().unwrap() {
                        list_except_last_lock.index -= 1;
                    }
                }
            }
        }
    }
}
