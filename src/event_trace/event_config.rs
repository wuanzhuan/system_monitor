use super::event_kernel;
use std::collections::HashMap;
use tracing::error;
use windows::core::GUID;
use windows::Win32::System::Diagnostics::Etw::CLASSIC_EVENT_ID;

pub struct Config {
    events_enables: Vec<EventEnable>,
    flag_map: HashMap<u32, Vec<usize>>,
    pub events_desc: &'static [event_kernel::EventsDescribe],
    pub events_opcode_map: HashMap<(GUID, u32), (usize, usize)>,
}

impl Config {
    pub fn new(events_desc: &'static [event_kernel::EventsDescribe]) -> Self {
        let mut event_enable = Vec::<EventEnable>::new();
        let mut events_opcode_map = HashMap::new();
        let mut flag_map: HashMap<u32, Vec<usize>> = HashMap::new();
        for (index_major, item) in events_desc.iter().enumerate() {
            let enable_minor = EventEnable {
                major: false,
                minors: vec![false; item.minors.len()],
            };
            event_enable.push(enable_minor);

            if let Some(vec) = flag_map.get_mut(&item.major.flag) {
                vec.push(index_major);
            }else {
                flag_map.insert(item.major.flag,  vec![index_major]);
            }

            for (index_minor, item_minor) in item.minors.iter().enumerate() {
                events_opcode_map
                    .insert((item.guid, item_minor.op_code), (index_major, index_minor));
            }
        }
        Self {
            events_enables: event_enable,
            events_desc,
            events_opcode_map,
            flag_map
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

    pub fn get_enable(&self, index_major: usize) -> &EventEnable {
        &self.events_enables[index_major]
    }

    pub fn set_enable(&mut self, index_major: usize, index_minor: Option<usize>, checked: bool) -> bool {
        let mut is_change = false;
        if let Some(index) = index_minor {
            if self.events_enables[index_major].minors[index] != checked {
                if !self.events_enables[index_major].major {
                    self.events_enables[index_major].major = true;
                }
                is_change = true;
                self.events_enables[index_major].minors[index] = checked;
            }
        } else {
            if self.events_enables[index_major].major != checked {
                is_change = true;
                self.events_enables[index_major].major = checked;
            }
        }
        return is_change;
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

    pub fn get_event_index_for_flag(&self, flag: u32) -> &[usize] {
        if let Some(vec) = self.flag_map.get(&flag){
            vec.as_slice()
        } else {
            &[]
        }
    }

    pub fn is_flag_enable(&self, flag: u32) -> bool {
        for item in self.get_event_index_for_flag(flag) {
            if self.events_enables[*item].major {
                return true;
            }
        }
        false
    }
}
pub struct EventEnable {
    pub major: bool,
    pub minors: Vec<bool>,
}
