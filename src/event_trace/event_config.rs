use super::event_kernel;


struct EventMask {
    major: bool,
    minor: Vec<bool>,
    event_desc: &'static event_kernel::EventsDescribe,
}

impl EventMask {
    pub fn new(index: usize) -> Self{
        let vec = vec![false; event_kernel::EVENTS_DESC[index].minors.len()];
        Self { major: false, minor: vec, event_desc: &event_kernel::EVENTS_DESC[index] }
    }
}