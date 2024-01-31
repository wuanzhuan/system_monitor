use tracing::error;
use super::event_kernel;
use windows::Win32::System::Diagnostics::Etw::CLASSIC_EVENT_ID;
use std::collections::HashMap;


pub struct Config {
    pub events_enables: Vec<EventEnable>,
    pub events_desc: &'static[event_kernel::EventsDescribe],
    pub events_enable_map: HashMap<&'static str, (usize, HashMap<&'static str, usize>)>
}

impl Config {
    pub fn new(events_desc: &'static[event_kernel::EventsDescribe]) -> Self {
        let mut event_enable = Vec::<EventEnable>::new();
        let mut events_name_map = HashMap::new();
        for (index, item) in events_desc.iter().enumerate() {
            let em = EventEnable{major: false, minors: vec![false; item.minors.len()]};
            event_enable.push(em);
            let mut minor_map = HashMap::new();
            for (index_minor, item_minor) in item.minors.iter().enumerate() {
                minor_map.insert(item_minor.name, index_minor);
            }
            events_name_map.insert(item.major.name, (index, minor_map));
        }
        Self{events_enables: event_enable, events_desc, events_enable_map: events_name_map}
    }

    #[allow(unused)]
    pub fn set_events_enables(&mut self, events_enables: &[EventEnable]) {
        if events_enables.len() != self.events_enables.len() {
            error!("invalid length of events_enables, expected: {}, found: {}", self.events_enables.len(), events_enables.len());
            return;
        }
        for (major_index, event_enable) in self.events_enables.iter_mut().enumerate() {
            if events_enables[major_index].minors.len() != event_enable.minors.len() {
                error!("invalid length of minor,index:{} expected: {}, found: {}", major_index, event_enable.minors.len(), events_enables[major_index].minors.len());
                return;
            }
            event_enable.major = events_enables[major_index].major;
            for (index_minor, minor) in event_enable.minors.iter_mut().enumerate() {
                *minor = events_enables[major_index].minors[index_minor];
            }

        }
    }

    pub fn get_group_mask(&self) -> event_kernel::PERFINFO_GROUPMASK {
        let mut gm = event_kernel::PERFINFO_GROUPMASK::new();
        gm.or_assign_with_groupmask(super::event_kernel::Major::NoSysConfig as u32);
        for (index, item) in self.events_enables.iter().enumerate() {
            if !item.major {
                continue;
            }
            gm.or_assign_with_groupmask(self.events_desc[index].major.flag);
        }
        gm
    }

    pub fn get_classic_event_id_vec(&self) -> (Vec::<CLASSIC_EVENT_ID>, usize) {
        let mut event_id_vec = Vec::<CLASSIC_EVENT_ID>::with_capacity(32);
        for (index, item) in self.events_enables.iter().enumerate() {
            if !item.major {
                continue;
            }
            let event_desc = &self.events_desc[index];
            for (index_minor, minor) in item.minors.iter().enumerate() {
                if !minor {
                    continue;
                }
                let id = CLASSIC_EVENT_ID{
                    EventGuid: event_desc.guid,
                    Type: event_desc.minors[index_minor].op_code as u8,
                    Reserved: [0u8; 7]
                };
                event_id_vec.push(id);
            }
        }
        let len = event_id_vec.len();

        (event_id_vec, std::mem::size_of::<CLASSIC_EVENT_ID>() * len)
    }
}
pub struct EventEnable {
    pub major: bool,
    pub minors: Vec<bool>,
}