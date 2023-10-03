use widestring::*;
use crate::third_extend::bytemuck::*;
pub use windows::core::PCWSTR;


pub trait AsPcwstr {
    fn as_pcwstr(&self) -> PCWSTR;
}

pub trait FromPcwstr {
    fn from_pcwstr<'a>(s: PCWSTR) -> &'a Self;
}

impl AsPcwstr for U16CStr {
    fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.as_ptr())
    }
}

impl FromPcwstr for U16CStr {
    fn from_pcwstr<'a>(s: PCWSTR) -> &'a Self {
        unsafe {
            U16CStr::from_ptr_str(s.0)
        }
    }
}

/// truncate when null
/// offset base bytes
#[inline]
pub fn u16cstr_from_bytes_truncate_offset(bytes: &[u8] , offset: u32) -> Option<&U16CStr>{
    if offset > 0 {
        U16CStr::from_slice_truncate(cast_slice_truncate(&bytes[(offset as usize)..])).ok()
    } else {
        None
    }
}
