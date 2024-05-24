use super::event_trace::{EventRecordDecoded, PropertyDecoded, StackWalk};
use crate::filter_expr::{Path, Value};
use crate::process_modules;
use crate::StackWalkInfo;
use anyhow::{anyhow, Result};
use slint::{Model, ModelRc, ModelTracker, SharedString, StandardListViewItem, VecModel};
use std::sync::{Arc, OnceLock};
use tracing::error;

#[derive(Clone)]
pub struct EventRecordModel {
    array: Arc<EventRecordDecoded>,
    stack_walk: OnceLock<Arc<StackWalk>>,
}

pub const COLUMN_NAMES: &[&str] = &[
    "datetime",
    "process_id",
    "thread_id",
    "event_name",
    "opcode_name",
    "properties",
];

impl EventRecordModel {
    pub fn new(event_record: EventRecordDecoded) -> Self {
        EventRecordModel {
            array: Arc::new(event_record),
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
                        let file_name = if let Some(offset) = module_info.file_name.rfind("\\") {
                            module_info
                                .file_name
                                .get(offset + 1..)
                                .unwrap_or("no_file_name")
                        } else {
                            module_info.file_name.as_str()
                        };
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
        match path.key.as_str() {
            "datetime" => {
                if let Value::I64(num) = value {
                    if *num == self.array.timestamp.0 {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            "process_id" => {
                if let Value::I64(num) = value {
                    if *num as u32 == self.array.process_id {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            "thread_id" => {
                if let Value::I64(num) = value {
                    if *num as u32 == self.array.thread_id {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            "event_name" => {
                if let Value::Str(num) = value {
                    if *num == self.array.event_name {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            "opcode_name" => {
                if let Value::Str(num) = value {
                    if *num == self.array.opcode_name {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                return Err(anyhow!("invalid value type"));
            }
            "properties" => {
                if let Value::Object(obj) = value {
                    if let Some(ref field) = path.field {
                        if let PropertyDecoded::Struct(ref properties) = self.array.properties {
                            if let PropertyDecoded::String(ref property_decoded_str) =
                                properties[field]
                            {
                                if let Value::Str(ref value_str) = obj[field] {
                                    if value_str == property_decoded_str {
                                        return Ok(true);
                                    }
                                    return Ok(false);
                                } else {
                                    return Err(anyhow!("The finding properties.{field}'s value's type is not Value::Str"));
                                }
                            } else {
                                return Err(anyhow!("The properties's filed type is not string"));
                            }
                        }
                    } else {
                        return Err(anyhow!("Not assign filed for properties"));
                    }
                }
                return Err(anyhow!("invalid value type"));
            }
            _ => Err(anyhow!("no this column name")),
        }
    }
}

impl Model for EventRecordModel {
    type Data = StandardListViewItem;

    fn row_count(&self) -> usize {
        COLUMN_NAMES.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        if row >= COLUMN_NAMES.len() {
            None
        } else {
            match row {
                0 => Some(StandardListViewItem::from(SharedString::from(
                    self.array.timestamp.to_datetime_detail(),
                ))),
                1 => Some(StandardListViewItem::from(SharedString::from(
                    (self.array.process_id as i32).to_string(),
                ))),
                2 => Some(StandardListViewItem::from(SharedString::from(
                    (self.array.thread_id as i32).to_string(),
                ))),
                3 => Some(StandardListViewItem::from(SharedString::from(
                    self.array.event_name.to_string(),
                ))),
                4 => Some(StandardListViewItem::from(SharedString::from(
                    self.array.opcode_name.to_string(),
                ))),
                5 => Some(StandardListViewItem::from(SharedString::from(
                    serde_json::to_string(&self.array.properties).unwrap_or_default(),
                ))),
                _ => None,
            }
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
