use chrono::*;
use serde::{Serialize, Serializer};
use std::ops::Sub;

/// https://learn.microsoft.com/zh-CN/windows/win32/api/minwinbase/ns-minwinbase-filetime
#[derive(Debug, Clone, Copy)]
pub struct TimeStamp(pub i64);

impl TimeStamp {
    pub fn to_datetime_local(&self) -> DateTime::<Local> {
        let duration = Utc.ymd(1970, 1, 1) - Utc.ymd(1601, 1, 1);
        let dt_utc = Utc.timestamp_millis(self.0 / 10 / 1000 - duration.num_milliseconds());
        DateTime::<Local>::from(dt_utc)
    }

    pub fn to_string_detail(&self) -> String {
        let dt = self.to_datetime_local();
        format!("{}({})", self.0, dt.to_string())
    }

    pub fn to_datetime_detail(&self) -> String {
        let dt = self.to_datetime_local();
        format!("{}({})", dt.to_string(), self.0)
    }
}

impl std::string::ToString for TimeStamp {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Sub for TimeStamp {
    type Output = i64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Serialize for TimeStamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        serializer.serialize_str(self.to_string_detail().as_str())
    }
}

#[allow(unused)]
pub fn get_path_from_commandline(commandline: &str) -> String {
    let mut is_in_quotation_mark = false;
    let mut is_escape_character_prefix = false;
    let mut string = String::with_capacity(commandline.len());
    for ch in commandline.chars() {
        if ch == '\\' {
            if !is_escape_character_prefix {
                is_escape_character_prefix = true;
                continue;
            }
        }
        if is_escape_character_prefix {
            is_escape_character_prefix = false;
        }
        if ch == '"' {
            is_in_quotation_mark = !is_in_quotation_mark;
            continue;
        }
        if ch == ' ' {
            if !is_in_quotation_mark {
                break;
            }
        }
        string.push(ch);
    }
    string
}


#[cfg(test)]
mod tests {
    #[test]
    fn get_path_from_commandline() {
        let s = super::get_path_from_commandline(r#"\"C:\\Program Files\\Git\\cmd\\git.exe\" show --textconv :src/event_trace/mod.rs"#);
        assert_eq!(s, String::from(r"C:\Program Files\Git\cmd\git.exe"));
    }
}