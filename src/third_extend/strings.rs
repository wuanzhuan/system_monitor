use crate::third_extend::bytemuck::*;
use tracing::debug;
use widestring::*;
pub use windows::core::PCWSTR;

pub trait AsPcwstr {
    fn as_pcwstr(&self) -> PCWSTR;
}

pub trait FromPcwstr {
    #[allow(unused)]
    fn from_pcwstr<'a>(s: PCWSTR) -> &'a Self;
}

impl AsPcwstr for U16CStr {
    fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.as_ptr())
    }
}

impl FromPcwstr for U16CStr {
    fn from_pcwstr<'a>(s: PCWSTR) -> &'a Self {
        unsafe { U16CStr::from_ptr_str(s.0) }
    }
}

/// [`truncate`] when encounter null
/// [`offset`] in bytes for bytes
#[inline]
pub fn u16cstr_from_bytes_truncate_offset(bytes: &[u8], offset: u32) -> Option<&U16CStr> {
    if offset > 0 {
        if offset as usize > bytes.len() {
            let backtrace = std::backtrace::Backtrace::force_capture();
            debug!(
                "Offset: {offset} is out of len: {}\n{}",
                bytes.len(),
                backtrace
            );
            None
        } else {
            U16CStr::from_slice_truncate(cast_slice_truncate(&bytes[(offset as usize)..])).ok()
        }
    } else {
        None
    }
}

pub trait StringEx {
    fn starts_with_case_insensitive(&self, pattern: &str) -> bool;
}

impl StringEx for String {
    fn starts_with_case_insensitive(&self, pattern: &str) -> bool {
        let mut chars_self = self.chars();
        let chars_pattern = pattern.chars();
        if self.len() < pattern.len() {
            return false;
        }
        for ch in chars_pattern {
            if let Some(char_self) = chars_self.next() {
                if ch.to_ascii_lowercase() != char_self.to_ascii_lowercase() {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::third_extend::strings::StringEx;

    #[test]
    fn starts_with_case_insensitive() {
        let s = String::from("\\systemRoot\\windows");
        assert!(s.starts_with_case_insensitive("\\SystemRoot\\"));
        assert!(s.starts_with_case_insensitive("\\Systemroot\\"));
        assert!(!s.starts_with_case_insensitive("\\SystemRootx\\"));
    }
}
