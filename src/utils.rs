use chrono::*;
use serde::{Serialize, Serializer};


/// https://learn.microsoft.com/zh-CN/windows/win32/api/minwinbase/ns-minwinbase-filetime
#[derive(Debug)]
pub struct TimeStamp(pub i64);

impl TimeStamp {
    pub fn to_datetime_local(&self) -> DateTime::<Local> {
        let duration = Utc.ymd(1970, 1, 1) - Utc.ymd(1601, 1, 1);
        let dt_utc = Utc.timestamp_millis(self.0 / 10 / 1000 - duration.num_milliseconds());
        DateTime::<Local>::from(dt_utc)
    }
}

impl std::string::ToString for TimeStamp {
    fn to_string(&self) -> String {
        let dt = self.to_datetime_local();
        dt.to_string()
    }
}

impl Serialize for TimeStamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}