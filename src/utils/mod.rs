use anyhow::{anyhow, Result};
use chrono::*;
use serde::{Serialize, Serializer};
use std::{env, ops::Sub, convert::From};
use windows::Win32::Foundation::FILETIME;


/// https://learn.microsoft.com/zh-CN/windows/win32/api/minwinbase/ns-minwinbase-filetime
#[derive(Debug, Clone, Copy)]
pub struct TimeStamp(pub i64);

impl TimeStamp {
    pub fn to_datetime_local(&self) -> DateTime<Local> {
        let duration = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap()
            - Utc.with_ymd_and_hms(1601, 1, 1, 0, 0, 0).unwrap();
        let dt_utc = Utc
            .timestamp_millis_opt(self.0 / 10 / 1000 - duration.num_milliseconds())
            .single()
            .unwrap_or_default();
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

    // the qpc is QueryPerformanceCounter
    pub fn from_qpc(count: u64, start_time: Self, perf_freq: i64) -> Self {
        let duration = count as f64 * 10000000.0 / perf_freq as f64;
        Self(start_time.0 + duration as i64)
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
        S: Serializer,
    {
        serializer.serialize_str(self.to_string_detail().as_str())
    }
}

impl From<FILETIME> for TimeStamp {
    fn from(value: FILETIME) -> Self {
        let mut int = value.dwHighDateTime as u64;
        int = (int << 32) + value.dwLowDateTime as u64;
        TimeStamp(int as i64)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeDateStamp(pub u32);

impl TimeDateStamp {
    pub fn to_datetime_local(&self) -> DateTime<Local> {
        let dt_utc = Utc
            .timestamp_opt(self.0 as i64, 0)
            .single()
            .unwrap_or_default();
        DateTime::<Local>::from(dt_utc)
    }

    pub fn to_string_detail(&self) -> String {
        let dt = self.to_datetime_local();
        format!("{}({})", self.0, dt.to_string())
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

//no \ at end
pub fn get_exe_dir() -> Result<String> {
    match env::current_exe() {
        Ok(path) => {
            if let Some(path_str) = path.to_str() {
                if let Some(index) = path_str.rfind("\\") {
                    Ok(path_str[..index].to_string())
                } else {
                    Err(anyhow!("Can't find \\ in path: {path:?}"))
                }
            } else {
                Err(anyhow!("Can't convert to str: {path:?}"))
            }
        }
        Err(e) => Err(anyhow!("Failed to env::current_exe: {e}")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_path_from_commandline() {
        let s = super::get_path_from_commandline(
            r#"\"C:\\Program Files\\Git\\cmd\\git.exe\" show --textconv :src/event_trace/mod.rs"#,
        );
        assert_eq!(s, String::from(r"C:\Program Files\Git\cmd\git.exe"));
    }

    #[test]
    fn get_exe_dir() {
        let r = super::get_exe_dir();
        assert!(r.is_ok());
    }
}
