use super::event_kernel;
use std::collections::HashMap;
use tracing::error;
use windows::core::GUID;
use windows::Win32::System::Diagnostics::Etw::CLASSIC_EVENT_ID;

pub struct Config {
    pub events_enables: Vec<EventEnable>,
    pub events_desc: &'static [event_kernel::EventsDescribe],
    pub events_name_map: HashMap<(&'static str, &'static str), (usize, usize)>,
    pub events_opcode_map: HashMap<(GUID, u32), (usize, usize)>,
}

impl Config {
    pub fn new(events_desc: &'static [event_kernel::EventsDescribe]) -> Self {
        let mut event_enable = Vec::<EventEnable>::new();
        let mut events_name_map = HashMap::new();
        let mut events_opcode_map = HashMap::new();
        for (index_major, item) in events_desc.iter().enumerate() {
            let enable_minor = EventEnable {
                major: false,
                minors: vec![false; item.minors.len()],
            };
            event_enable.push(enable_minor);

            for (index_minor, item_minor) in item.minors.iter().enumerate() {
                events_name_map.insert(
                    (item.major.name, item_minor.name),
                    (index_major, index_minor),
                );
                events_opcode_map
                    .insert((item.guid, item_minor.op_code), (index_major, index_minor));
            }
        }
        Self {
            events_enables: event_enable,
            events_desc,
            events_name_map,
            events_opcode_map,
        }
    }

    #[allow(unused)]
    pub fn set_events_enables(&mut self, events_enables: &[EventEnable]) {
        if events_enables.len() != self.events_enables.len() {
            error!(
                "invalid length of events_enables, expected: {}, found: {}",
                self.events_enables.len(),
                events_enables.len()
            );
            return;
        }
        for (major_index, event_enable) in self.events_enables.iter_mut().enumerate() {
            if events_enables[major_index].minors.len() != event_enable.minors.len() {
                error!(
                    "invalid length of minor,index:{} expected: {}, found: {}",
                    major_index,
                    event_enable.minors.len(),
                    events_enables[major_index].minors.len()
                );
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
        gm.or_assign_with_groupmask(super::event_kernel::Major::Process as u32);
        gm.or_assign_with_groupmask(super::event_kernel::Major::ImageLoad as u32);
        for (index, item) in self.events_enables.iter().enumerate() {
            if !item.major {
                continue;
            }
            gm.or_assign_with_groupmask(self.events_desc[index].major.flag);
        }
        gm
    }

    pub fn get_classic_event_id_vec(&self) -> (Vec<CLASSIC_EVENT_ID>, usize) {
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
                let id = CLASSIC_EVENT_ID {
                    EventGuid: event_desc.guid,
                    Type: event_desc.minors[index_minor].op_code as u8,
                    Reserved: [0u8; 7],
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
