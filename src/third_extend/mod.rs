pub mod strings;

pub mod bytemuck {
    use bytemuck::*;
    pub fn cast_slice_truncate<A: NoUninit, B: AnyBitPattern>(a: &[A]) -> &[B] {
        let new_len = core::mem::size_of_val(a) / core::mem::size_of::<B>();
        unsafe { core::slice::from_raw_parts(a.as_ptr() as *const B, new_len) }
    }
}

use serde::{Serialize, Serializer};
use windows::core::GUID;

#[derive(Debug, Default)]
pub struct Guid(pub GUID);

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{:?}", self.0).as_str())
    }
}
