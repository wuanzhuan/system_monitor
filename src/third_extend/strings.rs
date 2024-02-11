use widestring::*;
use crate::third_extend::bytemuck::*;
pub use windows::core::PCWSTR;
use tracing::{debug};


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

/// [`truncate`] when encounter null
/// [`offset`] in bytes for bytes
#[inline]
pub fn u16cstr_from_bytes_truncate_offset(bytes: &[u8] , offset: u32) -> Option<&U16CStr>{
    if offset > 0 {
        if offset as usize > bytes.len() {
            let backtrace = std::backtrace::Backtrace::force_capture();
            debug!("Offset: {offset} is out of len: {}\n{}", bytes.len(), backtrace);
            None
        } else {
            U16CStr::from_slice_truncate(cast_slice_truncate(&bytes[(offset as usize)..])).ok()
        }
    } else {
        None
    }
}

pub fn slice_to_string_uncheck(slice: &[u8]) -> String {
    let mut vec = Vec::<u8>::with_capacity(slice.len() * 2);
    unsafe{ vec.set_len(slice.len() * 2); }
    for (index, item) in slice.iter().enumerate() {
        if item.is_ascii() {
            vec[2*index] = b' ';
            vec[2*index + 1] = *item;

        } else {
            let str_entry = vec.get_mut(2*index..(2*index+2)).unwrap();
            str_entry.copy_from_slice(format!("{:x}", *item).as_bytes());
        }
    }
    unsafe{ String::from_utf8_unchecked(vec) }
}
