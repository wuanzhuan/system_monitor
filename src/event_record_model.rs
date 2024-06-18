use super::event_trace::{EventRecordDecoded, PropertyDecoded, StackWalk};
use crate::filter::{Path, Value};
use crate::process_modules;
use crate::StackWalkInfo;
use anyhow::{anyhow, Result, Error};
use slint::{Model, ModelRc, ModelTracker, SharedString, StandardListViewItem, VecModel};
use std::{
    sync::{Arc, OnceLock},
    str::FromStr
};
use tracing::error;
use strum::{VariantArray, AsRefStr};


#[derive(Clone)]
pub struct EventRecordModel {
    pub array: Arc<EventRecordDecoded>,
    pub process_path: String,
    stack_walk: OnceLock<Arc<StackWalk>>,
}

impl EventRecordModel {
    pub fn new(event_record: EventRecordDecoded, process_path: String) -> Self {
        EventRecordModel {
            array: Arc::new(event_record),
            process_path,
            stack_walk: OnceLock::new(),
        }
    }

    pub fn data_detail(&self) -> Option<SharedString> {
        Some(SharedString::from(
            serde_json::to_string_pretty(&*self.array).unwrap_or_default(),
        ))
    }

    /// Returns true if the `stack_walk` is None
    pub fn set_stack_walk(&self, sw: StackWalk) {
        let stack_process = sw.stack_process;
        let stack_thread = sw.stack_thread as i32;
        let event_timestamp = sw.event_timestamp;
        if self.stack_walk.set(Arc::new(sw)).is_err() {
            let process_id = self.array.process_id as i32;
            let thread_id = self.array.thread_id as i32;
            let timestamp = self.array.timestamp.0;
            error!(
                "Stalkwalk event had been set! {process_id}:{thread_id}:{timestamp}  {}:{}:{}",
                stack_process, stack_thread as i32, event_timestamp
            );
        }
    }

    pub fn stack_walk(&self) -> StackWalkInfo {
        if let Some(sw) = self.stack_walk.get() {
            let vec = VecModel::<SharedString>::default();
            for item in sw.stacks.iter() {
                let s = if let Some(relative) = item.1.relative {
                    if let Some(module_info) = process_modules::get_module_info_by_id(relative.0) {
                        let file_name = module_info.get_module_name();
                        format!(
                            "{}: {:#x} {}+{:#x}",
                            item.0, item.1.raw, file_name, relative.1
                        )
                    } else {
                        format!(
                            "{}: {:#x} {}+{:#x}",
                            item.0, item.1.raw, relative.0, relative.1
                        )
                    }
                } else {
                    format!("{}: {:#x}", item.0, item.1.raw)
                };
                vec.push(SharedString::from(s.as_str()))
            }
            StackWalkInfo {
                event_timestamp: SharedString::from(sw.event_timestamp.to_string()),
                process_id: SharedString::from(format!("{}", sw.stack_process as i32)),
                thread_id: SharedString::from(format!("{}", sw.stack_thread as i32)),
                stacks: ModelRc::<SharedString>::new(vec),
            }
        } else {
            StackWalkInfo::default()
        }
    }

    pub fn find_by_path_value(&self, path: &Path, value: &Value) -> Result<bool> {
        match path.key {
            Columns::Datetime => {
                if let Value::I64(num) = value {
                    if *num == self.array.timestamp.0 {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::ProcessName => {
                if let Value::Str(string) = value {
                    if self.process_path.to_ascii_lowercase() == string.to_ascii_lowercase() {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::ProcessId => {
                if let Value::I64(num) = value {
                    if *num as u32 == self.array.process_id {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::ThreadId => {
                if let Value::I64(num) = value {
                    if *num as u32 == self.array.thread_id {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::EventName => {
                if let Value::Str(string) = value {
                    if self.array.get_event_display_name().to_ascii_lowercase() == string.to_ascii_lowercase() {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::OpcodeName => {
                if let Value::Str(string) = value {
                    if self.array.opcode_name.to_ascii_lowercase() == string.to_ascii_lowercase() {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            Columns::Properties => {
                if let Some(ref field) = path.field {
                    if let Value::Str(ref value_str) = value {
                        if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                            if let PropertyDecoded::String(ref property_field_str) =
                                properties[field]
                            {
                                return Ok(value_str.to_ascii_lowercase()
                                    == property_field_str.to_ascii_lowercase());
                            } else {
                                return Err(anyhow!("The properties's {field} type is not string"));
                            }
                        } else {
                            return Err(anyhow!(
                                "The properties of {}-{} is not a struct!",
                                self.array.get_event_display_name(),
                                self.array.opcode_name
                            ));
                        }
                    } else {
                        return Err(anyhow!(
                            "The finding properties.{field}'s value's type is not Value::Str"
                        ));
                    }
                } else {
                    return Err(anyhow!("Not assign field for properties"));
                }
            }
        }
    }

    pub fn find_by_value(&self, value: &Value) -> Result<bool> {
        match value {
            Value::Str(v) => {
                if self
                    .array
                    .get_event_display_name()
                    .to_ascii_lowercase()
                    .contains(v.to_ascii_lowercase().as_str())
                {
                    return Ok(true);
                }
                if self
                    .array
                    .opcode_name
                    .to_ascii_lowercase()
                    .contains(v.to_ascii_lowercase().as_str())
                {
                    return Ok(true);
                }
                if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                    for (key, value) in properties.iter() {
                        if key
                            .to_ascii_lowercase()
                            .contains(v.to_ascii_lowercase().as_str())
                        {
                            return Ok(true);
                        }
                        if let PropertyDecoded::String(ref value_str) = value {
                            if value_str
                                .to_ascii_lowercase()
                                .contains(v.to_ascii_lowercase().as_str())
                            {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            Value::I64(v) => {
                if *v == self.array.timestamp.0 {
                    return Ok(true);
                }
                if *v == self.array.process_id as i64 {
                    return Ok(true);
                }
                if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                    for (key, value) in properties.iter() {
                        if key.to_ascii_lowercase().contains(v.to_string().as_str()) {
                            return Ok(true);
                        }
                        if let PropertyDecoded::String(ref value_str) = value {
                            if value_str
                                .to_ascii_lowercase()
                                .contains(v.to_string().as_str())
                            {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            Value::Num(v) => {
                if *v == self.array.process_id as f64 {
                    return Ok(true);
                }
                if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                    for (key, value) in properties.iter() {
                        if key.to_ascii_lowercase().contains(v.to_string().as_str()) {
                            return Ok(true);
                        }
                        if let PropertyDecoded::String(ref value_str) = value {
                            if value_str
                                .to_ascii_lowercase()
                                .contains(v.to_string().as_str())
                            {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            _ => Err(anyhow!("Not supported value type")),
        }
    }

    pub fn get_key_by_paths(&self, paths: &[Path]) -> Result<String> {
        let mut s = String::with_capacity(32);
        for path in paths {
            match path.key {
                Columns::Datetime => {
                    s.push_str(self.array.timestamp.0.to_string().as_str());
                }
                Columns::ProcessName => {
                    s.push_str(self.process_path.as_str());
                }
                Columns::ProcessId => {
                    s.push_str(self.array.process_id.to_string().as_str());
                }
                Columns::ThreadId => {
                    s.push_str(self.array.thread_id.to_string().as_str());
                }
                Columns::EventName => {
                    s.push_str(self.array.get_event_display_name());
                }
                Columns::OpcodeName => {
                    s.push_str(self.array.opcode_name.as_str());
                }
                Columns::Properties => {
                    if let Some(ref field) = path.field {
                        if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                            if let PropertyDecoded::String(ref property_field_str) =
                                properties[field]
                            {
                                s.push_str(property_field_str);
                            } else {
                                return Err(anyhow!(
                                    "The properties's {field} type is not string, {:?}",
                                    properties[field]
                                ));
                            }
                        } else {
                            return Err(anyhow!(
                                "The properties of {}-{} is not a struct!",
                                self.array.get_event_display_name(),
                                self.array.opcode_name
                            ));
                        }
                    } else {
                        return Err(anyhow!("Not assign field for properties"));
                    }
                }
            }
        }
        Ok(s)
    }

    pub fn get_process_name(&self) -> &str {
        process_modules::get_file_name_from_path(self.process_path.as_str())
    }

}

impl Model for EventRecordModel {
    type Data = StandardListViewItem;

    fn row_count(&self) -> usize {
        <Columns as VariantArray>::VARIANTS.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row >= <Columns as VariantArray>::VARIANTS.len() {
            return None;
        }
        match <Columns as VariantArray>::VARIANTS[row] {
            Columns::Datetime => Some(StandardListViewItem::from(SharedString::from(
                self.array.timestamp.to_datetime_detail(),
            ))),
            Columns::ProcessName => Some(StandardListViewItem::from(SharedString::from(
                self.get_process_name()
            ))),
            Columns::ProcessId => Some(StandardListViewItem::from(SharedString::from(
                (self.array.process_id as i32).to_string(),
            ))),
            Columns::ThreadId => Some(StandardListViewItem::from(SharedString::from(
                (self.array.thread_id as i32).to_string(),
            ))),
            Columns::EventName => Some(StandardListViewItem::from(SharedString::from(
                self.array.get_event_display_name().to_string(),
            ))),
            Columns::OpcodeName => Some(StandardListViewItem::from(SharedString::from(
                self.array.opcode_name.to_string(),
            ))),
            Columns::Properties => Some(StandardListViewItem::from(SharedString::from(
                serde_json::to_string(&self.array.properties).unwrap_or_default(),
            ))),
        }
    }

    fn set_row_data(&self, #[allow(unused)] row: usize, #[allow(unused)] data: Self::Data) {
        // if set don't forget to call row_changed
        //self.notify.row_changed(row);
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn core::any::Any {
        // a typical implementation just return `self`
        self
    }
}

#[derive(Debug, Clone, PartialEq, VariantArray, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Datetime,
    ProcessName,
    ProcessId,
    ThreadId,
    EventName,
    OpcodeName,
    Properties,
}

impl FromStr for Columns {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for column in Columns::VARIANTS {
            if s == column.as_ref() {
                return Ok(column.clone());
            }
        }
        Err(anyhow!("invalid Columns string: {s}"))
    }
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn column_as_str() {
        assert_eq!(Columns::Datetime.as_ref(), "datetime");
        assert_eq!(Columns::ProcessId.as_ref(), "process_id");
        assert_eq!(Columns::ThreadId.as_ref(), "thread_id");
        assert_eq!(Columns::EventName.as_ref(), "event_name");
        assert_eq!(Columns::OpcodeName.as_ref(), "opcode_name");
        assert_eq!(Columns::Properties.as_ref(), "properties");
    }
}
