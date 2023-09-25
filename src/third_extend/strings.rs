use widestring::*;
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
