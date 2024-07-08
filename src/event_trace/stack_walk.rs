use crate::utils::TimeStamp;
use linked_hash_map::LinkedHashMap;
use std::collections::VecDeque;
use tracing::{debug, error};

pub struct StackWalkMap<T: Clone> {
    events_map: EventMap<T>,
    events_map_thread_minus_1: EventMultipleMap<T>,
    events_map_for_second_sw: EventMap<T>,
    events_map_for_second_sw_thread_minus_1: EventMultipleMap<T>,
}

impl<T: Clone> StackWalkMap<T> {
    pub fn new(capacity: usize, max_count: usize, num_seconds: i64) -> Self {
        Self {
            events_map: EventMap::new(capacity, max_count, num_seconds, true),
            events_map_thread_minus_1: EventMultipleMap::new(capacity, max_count, num_seconds, true),
            events_map_for_second_sw: EventMap::new(capacity, max_count, num_seconds, false),
            events_map_for_second_sw_thread_minus_1: EventMultipleMap::new(capacity, max_count, num_seconds, false),
        }
    }

    // debug_msg: event info i.e. event name opcode name
    pub fn insert(&mut self, key: (u32, i64), value: T, debug_msg: String) {
        if key.0 == -1i32 as u32 {
            self.events_map_thread_minus_1.insert(key.1, (value, debug_msg), key.1);
        } else {
            if let Some(old_value) = self.events_map.try_insert(key, (value, debug_msg.clone()), key.1) {
                error!("Event's key for stack walk is repeated. thread_id: {} timestamp: {}: new: {debug_msg} old: {}", key.0 as i32, key.1, old_value.1);
            }
        }
    }

    pub fn remove(
        &mut self,
        key: &(u32, i64),
        current_timestamp: i64,
    ) -> Option<(T, /*is_from_second_sw_map*/ bool)> {
        if let Some((value, debug_msg)) = self.events_map.remove(key) {
            self.events_map_for_second_sw
                .insert(*key, (value.clone(), debug_msg), current_timestamp);
            return Some((value, false));
        }

        if let Some((value, debug_msg)) = self.events_map_thread_minus_1.remove(key.1) {
            self.events_map_for_second_sw_thread_minus_1
                .insert(key.1, (value.clone(), debug_msg), current_timestamp);
            return Some((value, false));
        }

        if let Some((value, _)) = self.events_map_for_second_sw.remove(key) {
            return Some((value, true));
        }

        if let Some((value, _)) = self.events_map_for_second_sw_thread_minus_1.remove( key.1) {
            return Some((value, true));
        }

        None
    }

    pub fn clear(&mut self) {
        self.events_map.clear();
        self.events_map_thread_minus_1.clear();
        self.events_map_for_second_sw.clear();
        self.events_map_for_second_sw_thread_minus_1.clear();
    }
}

struct EventMap<T: Clone>{
    map: LinkedHashMap<(/*event thread_id*/ u32, /*event timestamp*/ i64), (T, /*debug_msg*/ String)>,
    max_count: usize,
    num_seconds: i64,
    is_trace: bool,
}

impl<T: Clone> EventMap<T> {
    fn new(capacity: usize, max_count: usize, num_seconds: i64, is_trace: bool) -> Self {
        Self{
            map: LinkedHashMap::with_capacity(capacity),
            max_count,
            num_seconds,
            is_trace
        }
    }

    fn insert(&mut self, key: (u32, i64), value: (T, String), current_timestamp: i64) -> Option<(T, String)>{
        let old = self.map.insert(key, value);
        // clear events_map's item that is hold too long. Avoid map being too large
        self.clear_front(current_timestamp);
        old
    }

    // If the map already had this key present, nothing is updated and return Some(old). otherwise None
    fn try_insert(&mut self, key: (u32, i64), value: (T, String), current_timestamp: i64) -> Option<(T, String)>{
        if let Some(old_value) = self.map.get(&key) {
            return Some((old_value.0.clone(), old_value.1.clone()));
        }
        let _ = self.map.insert(key, value);
        // clear events_map's item that is hold too long. Avoid map being too large
        self.clear_front(current_timestamp);
        None
    }

    fn remove(
        &mut self,
        key: &(u32, i64)
    ) -> Option<(T, String)> {
        self.map.remove(key)
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn clear_front(
        &mut self,
        current_timestamp: i64
    ) {
        let max_count = if self.max_count < self.map.len() {
            self.max_count
        } else {
            self.map.len()
        };
        for _index in 0..max_count {
            if let Some((key, _value)) = self.map.front() {
                let dt_prev = TimeStamp(key.1).to_datetime_local();
                let duration = TimeStamp(current_timestamp).to_datetime_local() - dt_prev;
                if duration.num_seconds() <= self.num_seconds {
                    break;
                }
            } else {
                break;
            }
            let (key, (_value, debug_msg)) = self.map.pop_front().unwrap();
            if self.is_trace {
                debug!(
                    "Miss stack walk for the event: thread_id: {} timestamp: {}. {debug_msg}",
                    key.0 as i32, key.1
                )
            }
        }
    }
}

struct EventMultipleMap<T: Clone>{   
    map: LinkedHashMap</*event timestamp*/ i64, VecDeque<(T, /*debug_msg*/ String)>>, // the event's thread id is -1. may be repeated. 
    max_count: usize,
    num_seconds: i64,
    is_trace: bool,
}

impl<T: Clone> EventMultipleMap<T> {
    fn new(capacity: usize, max_count: usize, num_seconds: i64, is_trace: bool) -> Self {
        Self{
            map: LinkedHashMap::with_capacity(capacity),
            max_count,
            num_seconds,
            is_trace
        }
    }

    fn insert(&mut self, key: i64, value: (T, String), current_timestamp: i64) {
        if let Some(v) = self.map.get_mut(&key) {
            v.push_back(value);
        } else {
            let mut vd = VecDeque::with_capacity(1);
            vd.push_back(value);
            self.map.insert(key, vd);
        }

        // clear events_map's item that is hold too long. Avoid map being too large
        self.clear_front(current_timestamp);
    }

    fn remove(
        &mut self,
        key: i64
    ) -> Option<(T, String)> {
        let mut should_remove = false;
        let mut removed = None;
        if let Some(value ) = self.map.get_mut(&key) {
            removed = value.pop_front();
            if removed.is_none() {
                should_remove = true;
            }
        }
        if should_remove {
            self.map.remove(&key);
        }
        removed
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn clear_front(
        &mut self,
        current_timestamp: i64
    ) {
        let max_count = if self.max_count < self.map.len() {
            self.max_count
        } else {
            self.map.len()
        };
        for _index in 0..max_count {
            if let Some((key, _value)) = self.map.front() {
                let dt_prev = TimeStamp(*key).to_datetime_local();
                let duration = TimeStamp(current_timestamp).to_datetime_local() - dt_prev;
                if duration.num_seconds() <= self.num_seconds {
                    break;
                }
            } else {
                break;
            }
            let (key, value) = self.map.pop_front().unwrap();
            if self.is_trace {
                let vec: Vec<String> = value.into_iter().map(|item| {
                    item.1
                }).collect();
                debug!(
                    "Miss stack walk for the event: thread_id: -1 timestamp: {key}. {vec:#?}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StackWalkMap;

    #[test]
    fn remove() {
        let mut map = StackWalkMap::<()>::new(10, 10, 15);

        map.insert((-1i32 as u32, 133644663686383541), (), format!("test"));

        let r = map.remove(&(44876, 133644663686383541), 133644663686383541);

        assert!(r.is_some());
    }
}
