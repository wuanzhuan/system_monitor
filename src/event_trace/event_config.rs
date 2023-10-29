use tracing::error;
use super::event_kernel;
use windows::Win32::System::Diagnostics::Etw::CLASSIC_EVENT_ID;

pub struct Config {
    pub events_enables: Vec<EventEnable>,
    pub events_desc: &'static[event_kernel::EventsDescribe]
}

impl Config {
    pub fn new(events_desc: &'static[event_kernel::EventsDescribe]) -> Self {
        let mut event_enable = Vec::<EventEnable>::new();
        for item in events_desc.iter() {
            let em = EventEnable{major: false, minors: vec![false; item.minors.len()]};
            event_enable.push(em);
        }
        Self{events_enables: event_enable, events_desc}
    }

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
        for (index, item) in self.events_enables.iter().enumerate() {
            if !item.major {
                continue;
            }
            gm.or_assign_with_groupmask(self.events_desc[index].major.flag);
        }
        gm
    }

    pub fn get_classic_event_id_vec(&self) -> Vec::<CLASSIC_EVENT_ID> {
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

        event_id_vec
    }
}
pub struct EventEnable {
    pub major: bool,
    pub minors: Vec<bool>,
}

impl EventEnable {
    pub fn new(index: usize) -> Self {
        let vec = vec![false; event_kernel::EVENTS_DESC[index].minors.len()];
        Self { major: false, minors: vec }
    }
}