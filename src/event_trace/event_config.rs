use super::event_kernel;
use windows::Win32::System::Diagnostics::Etw::CLASSIC_EVENT_ID;

pub struct Config {
    pub mask_vec: Vec<EventMask>,
    pub events_desc: &'static[event_kernel::EventsDescribe]
}

impl Config {
    pub fn new(events_desc: &'static[event_kernel::EventsDescribe]) -> Self {
        let mut mask_vec = Vec::<EventMask>::new();
        for item in events_desc.iter() {
            let em = EventMask{major: false, minor: vec![false; item.minors.len()]};
            mask_vec.push(em);
        }
        Self{mask_vec, events_desc}
    }

    pub fn get_group_mask(&self) -> event_kernel::PERFINFO_GROUPMASK {
        let mut gm = event_kernel::PERFINFO_GROUPMASK::new();
        for (index, item) in self.mask_vec.iter().enumerate() {
            if !item.major {
                continue;
            }
            gm.or_assign_with_groupmask(self.events_desc[index].major.flag);
        }
        gm
    }

    pub fn get_classic_event_id_vec(&self) -> Vec::<CLASSIC_EVENT_ID> {
        let mut event_id_vec = Vec::<CLASSIC_EVENT_ID>::with_capacity(32);
        for (index, item) in self.mask_vec.iter().enumerate() {
            if !item.major {
                continue;
            }
            let event_desc = &self.events_desc[index];
            for (index_minor, minor) in item.minor.iter().enumerate() {
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
pub struct EventMask {
    pub major: bool,
    pub minor: Vec<bool>,
}

impl EventMask {
    pub fn new(index: usize) -> Self {
        let vec = vec![false; event_kernel::EVENTS_DESC[index].minors.len()];
        Self { major: false, minor: vec }
    }
}