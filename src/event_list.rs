use anyhow::{anyhow, Result};
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{linked_list::Cursor, LinkedList, LinkedListLink};
use parking_lot::{FairMutex, RwLock, RwLockWriteGuard};
use std::{
    cell::SyncUnsafeCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

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
            value,
        }
    }
}

intrusive_adapter!(NodeAdapter<T> = Arc<Node<T>>: Node<T> { link: LinkedListLink });

struct CursorSync<'a, T> {
    pub inner: Cursor<'a, NodeAdapter<T>>,
    pub index: usize,
}
unsafe impl<'a, T> Send for CursorSync<'a, T> {}
unsafe impl<'a, T> Sync for CursorSync<'a, T> {}

pub struct EventList<'a: 'static, T> {
    // the backing data, access by cursor
    list: SyncUnsafeCell<Box<LinkedList<NodeAdapter<T>>>>,
    list_len: AtomicUsize,
    reader_lock: RwLock<CursorSync<'a, T>>,
    push_back_lock: FairMutex<()>,
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<'a, T> EventList<'a, T> {
    pub fn new() -> Self {
        let list = SyncUnsafeCell::new(Box::new(LinkedList::<NodeAdapter<T>>::default()));
        let list_len = AtomicUsize::new(0);
        let reader_lock = RwLock::new(CursorSync {
            inner: unsafe { &mut *list.get() }.cursor(),
            index: 0,
        });
        let push_back_lock = FairMutex::new(());
        Self {
            list,
            list_len,
            reader_lock,
            push_back_lock,
        }
    }

    pub fn len(&self) -> usize {
        self.list_len.load(Ordering::Acquire)
    }

    pub fn get_by_index(&self, index_to: usize) -> Option<Arc<Node<T>>> {
        let mut reader_guard = self.reader_lock.write();

        let list_len = self.list_len.load(Ordering::Acquire);
        if index_to >= list_len {
            return None;
        }

        let cursor_index = if !reader_guard.inner.is_null() {
            reader_guard.index
        } else {
            list_len
        };

        if cursor_index == index_to {
            return reader_guard.inner.clone_pointer();
        } else if cursor_index < index_to {
            if (index_to - cursor_index) * 2 <= list_len {
                move_next_to_uncheck(&mut reader_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.push_back_lock.lock();
                let list_len = self.list_len.load(Ordering::Acquire);
                *reader_guard = CursorSync {
                    inner: unsafe { &*self.list.get() }.front(),
                    index: 0,
                };
                move_prev_to_uncheck(&mut reader_guard, index_to, list_len);
            }
        } else {
            if (cursor_index - index_to) * 2 <= list_len {
                move_prev_to_uncheck(&mut reader_guard, index_to, list_len);
            } else {
                let _push_back_guard = self.push_back_lock.lock();
                let list_len = self.list_len.load(Ordering::Acquire);
                *reader_guard = CursorSync {
                    inner: unsafe { &*self.list.get() }.back(),
                    index: list_len - 1,
                };
                move_next_to_uncheck(&mut reader_guard, index_to, list_len);
            }
        }
        return reader_guard.inner.clone_pointer();

        fn move_next_to_uncheck<'a, T>(
            reader_guard: &mut RwLockWriteGuard<'_, CursorSync<'_, T>>,
            index_to: usize,
            list_len: usize,
        ) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = reader_guard.inner.is_null();
                reader_guard.inner.move_next();
                if reader_guard.inner.is_null() {
                    if prev_is_null {
                        reader_guard.index = 0;
                        break;
                    }
                    reader_guard.index = list_len;
                } else {
                    if prev_is_null {
                        reader_guard.index = 0;
                    } else {
                        reader_guard.index += 1;
                    }
                    if reader_guard.index == index_to {
                        break;
                    }
                }
            }
        }

        fn move_prev_to_uncheck<'a, T>(
            reader_guard: &mut RwLockWriteGuard<'_, CursorSync<'_, T>>,
            index_to: usize,
            list_len: usize,
        ) {
            assert!(list_len > 0);
            loop {
                let prev_is_null = reader_guard.inner.is_null();
                reader_guard.inner.move_prev();
                if reader_guard.inner.is_null() {
                    if prev_is_null {
                        reader_guard.index = 0;
                        break;
                    }
                    reader_guard.index = list_len;
                } else {
                    if prev_is_null {
                        reader_guard.index = list_len - 1;
                    } else {
                        reader_guard.index -= 1;
                    }
                    if reader_guard.index == index_to {
                        break;
                    }
                }
            }
        }
    }

    pub fn traversal(&self, mut cb: impl FnMut(&T) -> Result<bool>) -> Result<Vec<i32>> {
        let mut _reader_guard = self.reader_lock.read();
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
        if vec.is_empty() {
            Err(anyhow!("No item is find"))
        } else {
            Ok(vec)
        }
    }

    pub fn push(&self, value: Arc<Node<T>>) -> usize {
        let mut _push_back_guard = self.push_back_lock.lock();
        unsafe { &mut *self.list.get() }.push_back(value);
        let index = self.list_len.fetch_add(1, Ordering::Release);
        index
    }

    /// Remove the row at the given index from the model
    #[allow(unused)]
    pub fn remove(&self, node_arc: Arc<Node<T>>) {
        let mut cursor = unsafe { (&mut *self.list.get()).cursor_mut_from_ptr(node_arc.as_ref()) };
        if cursor.remove().is_some() {
            self.list_len.fetch_sub(1, Ordering::Release);
        }
        //notify
    }
}
