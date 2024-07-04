use crate::utils::TimeStamp;
use linked_hash_map::LinkedHashMap;
use tracing::warn;


pub struct StackWalkMap<T: Clone> {
    events_map: LinkedHashMap<(/*event thread_id*/ u32, /*event timestamp*/ i64), (T, /*debug_msg*/String)>,
    delay_remove_events_map: LinkedHashMap<(/*event thread_id*/ u32, /*event timestamp*/ i64), (T, /*debug_msg*/String)>,
    max_count: usize,
    num_seconds: i64,
}

impl<T: Clone> StackWalkMap<T> {
    pub fn new(capacity: usize, max_count: usize, num_seconds: i64) -> Self {
        Self {
            events_map: LinkedHashMap::with_capacity(capacity),
            delay_remove_events_map: LinkedHashMap::with_capacity(capacity),
            max_count,
            num_seconds,
        }
    }

    // debug_msg: event info i.e. event name opcode name
    pub fn insert(&mut self, key: (u32, i64), value: T, debug_msg: String) -> Option<(T, String)> {
        let old = self.events_map.insert(key, (value, debug_msg));
        // clear events_map's item that is hold too long. Avoid map being too large
        self.pop_front(false, key.1);
        old
    }

    pub fn remove(
        &mut self,
        key: &(u32, i64),
        current_timestamp: i64,
    ) -> Option<(T, /*is_from_delay_remove_map*/ bool)> {
        if let Some((value, debug_msg)) = self.events_map.remove(key) {
            self.delay_remove_events_map.insert(*key, (value.clone(), debug_msg));
            // clear delay_remove_events_map's item that is hold too long. Avoid map being too large
            self.pop_front(true, current_timestamp);
            return Some((value, false));
        }

        if let Some((value, debug_msg)) = self.events_map.remove(&(-1i32 as u32, key.1)) {
            self.delay_remove_events_map
                .insert((-1i32 as u32, key.1), (value.clone(), debug_msg));
            // clear delay_remove_events_map's item that is hold too long. Avoid map being too large
            self.pop_front(true, current_timestamp);
            return Some((value, false));
        }

        if let Some((value, _)) = self.delay_remove_events_map.remove(key) {
            return Some((value, true));
        }

        if let Some((value, _)) = self.delay_remove_events_map.remove(&(-1i32 as u32, key.1)) {
            return Some((value, true));
        }

        None
    }

    fn pop_front(&mut self, is_delay_remove_map: bool, current_timestamp: i64) {
        let map = if is_delay_remove_map {
            &mut self.delay_remove_events_map
        } else {
            &mut self.events_map
        };

        let max_count = if self.max_count < map.len() {
            self.max_count
        } else {
            map.len()
        };
        for _index in 0..max_count {
            let is_pop = if let Some((key, _value)) = map.front() {
                let dt_prev = TimeStamp(key.1).to_datetime_local();
                let duration = TimeStamp(current_timestamp).to_datetime_local() - dt_prev;
                if duration.num_seconds() > self.num_seconds {
                    true
                } else {
                    break;
                }
            } else {
                break;
            };

            if is_pop {
                let (key, (_value, debug_msg)) = map.pop_front().unwrap();
                if !is_delay_remove_map {
                    warn!(
                        "Miss stack walk for the event: thread_id: {} timestamp: {}. {debug_msg}",
                        key.0 as i32, key.1
                    )
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.events_map.clear();
        self.delay_remove_events_map.clear();
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
