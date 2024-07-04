use crate::utils::TimeStamp;
use linked_hash_map::LinkedHashMap;

pub struct StackWalkMap<T: Clone> {
    events_map: LinkedHashMap<(/*event thread_id*/ u32, /*event timestamp*/ i64), T>,
    delay_remove_events_map: LinkedHashMap<(/*event thread_id*/ u32, /*event timestamp*/ i64), T>,
}

impl<T: Clone> StackWalkMap<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            events_map: LinkedHashMap::with_capacity(capacity),
            delay_remove_events_map: LinkedHashMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, key: (u32, i64), value: T) -> Option<T> {
        self.events_map.insert(key, value)
    }

    pub fn remove(&mut self, key: &(u32, i64)) -> Option<(T, /*is_from_delay_remove_map*/ bool)> {
        if let Some(value) = self.events_map.remove(key) {
            self.delay_remove_events_map.insert(*key, value.clone());
            return Some((value, false));
        }

        if let Some(value) = self.events_map.remove(&(-1i32 as u32, key.1)) {
            self.delay_remove_events_map
                .insert((-1i32 as u32, key.1), value.clone());
            return Some((value, false));
        }

        if let Some(value) = self.delay_remove_events_map.remove(key) {
            return Some((value, true));
        }

        if let Some(value) = self.delay_remove_events_map.remove(&(-1i32 as u32, key.1)) {
            return Some((value, true));
        }

        None
    }

    pub fn pop_front(
        &mut self,
        is_delay_remove_map: bool,
        current_timestamp: i64,
        max_count: usize,
        num_seconds: i64,
        pop_callback: impl Fn(/*key*/(u32, i64), /*value*/T)
    ) {
        let map = if is_delay_remove_map {
            &mut self.delay_remove_events_map
        } else {
            &mut self.events_map
        };

        let max_count = if max_count < map.len() {
            max_count
        } else {
            map.len()
        };
        for _index in 0..max_count {
            let is_pop = if let Some((key, _value)) = map.front() {
                let dt_prev = TimeStamp(key.1).to_datetime_local();
                let duration = TimeStamp(current_timestamp).to_datetime_local() - dt_prev;
                if duration.num_seconds() > num_seconds {
                    true
                } else {
                    break;
                }
            } else {
                break;
            };

            if is_pop {
                let (key, value) = map.pop_front().unwrap();
                pop_callback(key, value);
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
        let mut map = StackWalkMap::<()>::new(10);

        map.insert((-1i32 as u32, 133644663686383541), ());

        let r = map.remove(&(44876, 133644663686383541));

        assert!(r.is_some());
    }
}
