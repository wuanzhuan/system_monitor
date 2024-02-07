use crate::third_extend::strings::*;
use std::{mem, slice};
use tracing::{error, warn};
use widestring::*;
use windows::{
    core::*,
    Win32::Foundation::*, 
    Win32::System::Diagnostics::Etw::*,
};
use serde::Serialize;
use linked_hash_map::LinkedHashMap;
use crate::utils::TimeStamp;
use crate::third_extend::Guid;


pub struct Decoder<'a>{
    event_record: &'a EVENT_RECORD,
    event_info: &'a TRACE_EVENT_INFO,
    event_info_slice: &'a [u8],
    property_info_array: &'a [EVENT_PROPERTY_INFO],
    user_data: &'a [u8],
    int_values: Vec<u16>,
    pointer_size: u32
}

impl<'a> Decoder<'a> {
    pub fn new(event_record: &'a EVENT_RECORD) -> Result<Self> {
        let header = &event_record.EventHeader;

        if (header.Flags & EVENT_HEADER_FLAG_TRACE_MESSAGE as u16) != 0 {
            return Err(Error::new(E_FAIL, HSTRING::from("this is wpp event, don't handle")));
        }

        let mut buffer_size = 4096u32;
        let mut event_info_vec = Vec::<u8>::with_capacity(buffer_size as usize);
        let mut event_info: &mut TRACE_EVENT_INFO = unsafe { mem::transmute(event_info_vec.as_mut_ptr()) };
        let mut status = unsafe {
            TdhGetEventInformation(
                event_record,
                None,
                Some(event_info as *mut TRACE_EVENT_INFO),
                &mut buffer_size,
            )
        };
        if status != ERROR_SUCCESS.0 {
            if status != ERROR_INSUFFICIENT_BUFFER.0 {
                warn!("Failded to TdhGetEventInformation: {}", status);
                return Err(Error::new(WIN32_ERROR(status).to_hresult(), HSTRING::from("Failed to TdhGetEventInformation")));
            }
            event_info_vec = Vec::<u8>::with_capacity(buffer_size as usize);
            event_info = unsafe { mem::transmute(event_info_vec.as_mut_ptr()) };
            status = unsafe {
                TdhGetEventInformation(
                    event_record,
                    None,
                    Some(event_info as *mut TRACE_EVENT_INFO),
                    &mut buffer_size,
                )
            };
            if status != ERROR_SUCCESS.0 {
                warn!("Failded to TdhGetEventInformation: {}", status);
                return Err(Error::new(WIN32_ERROR(status).to_hresult(), HSTRING::from("Failed to TdhGetEventInformation")));
            };
        };

        let event_info_slice = unsafe {slice::from_raw_parts(event_info_vec.as_ptr(), buffer_size as usize)};
        let property_info_array = unsafe {
            slice::from_raw_parts(event_info.EventPropertyInfoArray.as_ptr(), event_info.PropertyCount as usize)
        };
        let user_data = unsafe {
            slice::from_raw_parts(event_record.UserData as *const u8, event_record.UserDataLength as usize)
        };
        let int_values = vec![0u16; event_info.PropertyCount as usize];
        let pointer_size = if (header.Flags as u32 & EVENT_HEADER_FLAG_32_BIT_HEADER) != 0 {
            4u32
        } else if (header.Flags as u32 & EVENT_HEADER_FLAG_64_BIT_HEADER) != 0 {
            8u32
        } else {
            mem::size_of::<*const u8>() as u32
        };
        Ok(Self{
            event_record,
            event_info,
            event_info_slice,
            property_info_array,
            user_data,
            int_values,
            pointer_size
        })
    }
    pub fn decode(&mut self) -> Result<EventRecordDecoded>{
        let header = &self.event_record.EventHeader;
        let provider_id = Guid(header.ProviderId);
        let provider_name = u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.ProviderNameOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let level_name = u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.LevelNameOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let channel_name = u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.ChannelNameOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let keywords_name = u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.KeywordsNameOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let event_name = {
            let event_name_offset = unsafe { self.event_info.Anonymous1.EventNameOffset };
            if event_name_offset != 0 {
                u16cstr_from_bytes_truncate_offset(self.event_info_slice, event_name_offset)
                    .unwrap_or_default().to_string().unwrap_or_default()
            } else {
                u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.TaskNameOffset)
                    .unwrap_or_default().to_string().unwrap_or_default()
            }
        };
        let opcode_name =
            u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.OpcodeNameOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let event_message =
            u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.EventMessageOffset)
                .unwrap_or_default().to_string().unwrap_or_default();
        let provider_message =
            u16cstr_from_bytes_truncate_offset(self.event_info_slice, self.event_info.ProviderMessageOffset)
                .unwrap_or_default().to_string().unwrap_or_default();

        let properties = if is_string_event(header.Flags) {
            let s = 
            unsafe {
                U16CStr::from_ptr_truncate(
                    self.user_data.as_ptr() as *const u16,
                    (self.user_data.len() / 2) as usize,
                )
                .unwrap_or_default()
                .to_string().unwrap_or_default()
            };
            PropertyDecoded::String(s)
        } else {
            let mut user_data_index = 0u16;
            let r = self.decode_properties(
                0,
                self.event_info.TopLevelPropertyCount as u16,
                &mut user_data_index
            ).unwrap_or_default();
            PropertyDecoded::Struct(r)
        };
        Ok(EventRecordDecoded{
            provider_id,
            provider_name,
            level_name,
            channel_name,
            keywords_name,
            event_name,
            opcode_name,
            event_message,
            provider_message,
            process_id: header.ProcessId,
            thread_id: header.ThreadId,
            timestamp: TimeStamp(header.TimeStamp),
            properties
        })
    }

    fn decode_properties(
        &mut self,
        properties_array_begin: u16,
        properties_array_end: u16,
        user_data_index: &mut u16,
    ) -> Result<LinkedHashMap<String, PropertyDecoded>> {
        if properties_array_end >= self.property_info_array.len() as u16 {
            return Err(Error::new(E_FAIL, HSTRING::from(format!("Too larget properties_array_end: {properties_array_end} property_info_array len: {}", self.property_info_array.len()))));
        }
        let mut properties_object = LinkedHashMap::<String, PropertyDecoded>::new();
        let mut property_index = properties_array_begin;
        // top property may contain length/count
        while property_index < properties_array_end {
            let property_info = &self.property_info_array[property_index as usize];
            let property_name =
                u16cstr_from_bytes_truncate_offset(self.event_info_slice, property_info.NameOffset)
                    .unwrap_or_default();
            let property_name = if !property_name.is_empty() {
                property_name.to_string().unwrap_or_default()
            } else {
                format!("no name: {property_index}")
            };

            // If this property is a scalar integer, remember the value in case it
            // is needed for a subsequent property's length or count.
            if 0 == (property_info.Flags.0 & (PropertyStruct.0 | PropertyParamCount.0))
                && unsafe { property_info.Anonymous2.count } == 1
                && 0 == (property_info.Flags.0 & PropertyParamFixedCount.0) // if the event is compile by wdk earlier than wdk10, the PropertyParamFixedCount always is 0.so it is right too. 
            {
                let in_type = unsafe { property_info.Anonymous1.nonStructType.InType } as i32;
                if in_type == TDH_INTYPE_INT8.0 || in_type == TDH_INTYPE_UINT8.0 {
                    if self.user_data.len() - *user_data_index as usize >= 1 {
                        self.int_values[property_index as usize] = u8::from_le_bytes(
                            self.user_data[*user_data_index as usize..*user_data_index as usize + 1]
                                .try_into()
                                .unwrap(),
                        ) as u16;
                    }
                } else if in_type == TDH_INTYPE_INT16.0 || in_type == TDH_INTYPE_UINT16.0 {
                    if self.user_data.len() - *user_data_index as usize >= 2 {
                        self.int_values[property_index as usize] = u16::from_le_bytes(
                            self.user_data[*user_data_index as usize..*user_data_index as usize + 2]
                                .try_into()
                                .unwrap(),
                        );
                    }
                } else if in_type == TDH_INTYPE_INT32.0
                    || in_type == TDH_INTYPE_UINT32.0
                    || in_type == TDH_INTYPE_HEXINT32.0
                {
                    if self.user_data.len() - *user_data_index as usize >= 4 {
                        let v = u32::from_le_bytes(
                            self.user_data[*user_data_index as usize..*user_data_index as usize + 4]
                                .try_into()
                                .unwrap(),
                        );
                        self.int_values[property_index as usize] =
                            if v > 0xffff { 0xffff } else { v as u16 };
                    }
                }
            }
    
            let in_type = unsafe { property_info.Anonymous1.nonStructType.InType };
            let out_type = unsafe { property_info.Anonymous1.nonStructType.OutType };
            let length = unsafe { property_info.Anonymous3.length };
            let length_property_index = unsafe{ property_info.Anonymous3.lengthPropertyIndex };
            let prop_length = if out_type == TDH_OUTTYPE_IPV6.0 as u16
                && in_type == TDH_INTYPE_BINARY.0 as u16
                && length == 0
            {
                // special case for incorrectly-defined IPV6 addresses
                // reference: https://learn.microsoft.com/en-us/windows/win32/api/tdh/nf-tdh-tdhformatproperty#remarks
                // size of the Win32::Networking::WinSock::IN6_ADDR
                16
            } else if (property_info.Flags.0 & PropertyParamLength.0) != 0 {
                if length_property_index >= self.int_values.len() as u16 {
                    return Err(Error::new(E_FAIL, HSTRING::from(format!("index overflow: length_property_index: {length_property_index} array len: {}", self.int_values.len()))));
                }
                self.int_values[length_property_index as usize]
            // Look up the value of a previous property
            } else {
                length
            };
    
            let (array_count, is_array) = if (property_info.Flags.0 & PropertyParamCount.0) != 0 {
                let count_property_index = unsafe { property_info.Anonymous2.countPropertyIndex };
                if count_property_index >= property_index as u16 {
                    error!("invalid count_property_index: {count_property_index} index: {property_index} properties_array_end: {properties_array_end}");
                    return Err(Error::from(E_FAIL));
                }
                (self.int_values[count_property_index as usize], true) // Look up the value of a previous property
            } else {
                let count = unsafe { property_info.Anonymous2.count };
                if count == 1 {
                    if (property_info.Flags.0 & PropertyParamFixedCount.0) != 0 {
                        (1, true)
                    } else {
                        (1, false)
                    }
                } else {
                    (count, true)
                }
            };
            let is_struct = (property_info.Flags.0 & PropertyStruct.0) != 0;
    
            if is_struct {
                // If this property is a struct, recurse and print the child
                // properties.
                let struct_start_index =
                    unsafe { property_info.Anonymous1.structType.StructStartIndex };
                let num_of_struct_members =
                    unsafe { property_info.Anonymous1.structType.NumOfStructMembers };
                let r = self.decode_properties(
                    struct_start_index,
                    struct_start_index + num_of_struct_members,
                    user_data_index
                )?;
                properties_object.insert(property_name, PropertyDecoded::Struct(r));
            } else {
                let mut properties_array = Vec::<String>::new();
                // Treat non-array properties as arrays with one element.
                let mut array_index = 0;
                while array_index < array_count {
                    if (*user_data_index as usize) >= self.user_data.len() {
                        // it is a empty string when user_data is 
                        properties_array.append(&mut vec![String::from(""); (array_count - array_index) as usize]);
                        array_index = array_count;
                        continue;
                    }
                    // If the property has an associated map (i.e. an enumerated type),
                    // try to look up the map data. (If this is an array, we only need
                    // to do the lookup on the first iteration.)
                    let map_name_offset = unsafe { property_info.Anonymous1.nonStructType.MapNameOffset };
                    let mut _buffer_vec = Vec::<u8>::new();
                    let mut map_info: Option<*const EVENT_MAP_INFO> = None;
                    if map_name_offset != 0 && array_index == 0 {
                        if in_type == TDH_INTYPE_UINT8.0 as u16
                           || in_type == TDH_INTYPE_UINT16.0 as u16
                           || in_type == TDH_INTYPE_UINT32.0 as u16
                           || in_type == TDH_INTYPE_HEXINT32.0 as u16
                        {
                            let map_name = u16cstr_from_bytes_truncate_offset(self.event_info_slice, map_name_offset);
                            if let Some(map_name) = map_name {
                                if !map_name.is_empty() {
                                    let mut buffer_size = 1024u32;
                                    loop {
                                        _buffer_vec = Vec::<u8>::with_capacity(buffer_size as usize);
                                        let _map_info: &mut EVENT_MAP_INFO = unsafe { mem::transmute(_buffer_vec.as_ptr()) };
                                        map_info = Some(_map_info);
                                        let status = unsafe {
                                            TdhGetEventMapInformation(
                                                self.event_record,
                                                map_name.as_pcwstr(),
                                                Some(_map_info),
                                                &mut buffer_size,
                                            )
                                        };
                                        if status == ERROR_SUCCESS.0 {
                                            break;
                                        }
                                        if status == ERROR_INSUFFICIENT_BUFFER.0 {
                                            continue;
                                        }
                                        map_info = None;
                                        error!("Failed to TdhGetEventMapInformation: {}", status);
                                        break;
                                    }
                                }
                            }
                        }
                    };
    
                    let mut prop_buffer = Vec::<u16>::new();
                    
                    if 0 == prop_length && in_type == TDH_INTYPE_NULL.0 as u16 {
                        // TdhFormatProperty doesn't handle INTYPE_NULL.
                        prop_buffer.push(0);
                    } else if 0 == prop_length
                        && 0 != (property_info.Flags.0 & PropertyParamFixedLength.0)
                        && (in_type == TDH_INTYPE_UNICODESTRING.0 as u16
                            || in_type == TDH_INTYPE_ANSISTRING.0 as u16)
                    {
                        // TdhFormatProperty doesn't handle zero-length counted strings.
                        prop_buffer.push(0);
                    } else {
                        let out_type = if out_type == TDH_OUTTYPE_NOPRINT.0 as u16 {
                            TDH_OUTTYPE_NULL.0 as u16
                        } else {
                            out_type
                        };
    
                        let mut buffer_size = 1024u32;
                        prop_buffer.resize((buffer_size / 2) as usize, 0);
                        let mut userdataconsumed = 0u16;
                        loop {
                            let buffer = PWSTR::from_raw(prop_buffer.as_mut_ptr());
                            let status = unsafe {
                                TdhFormatProperty(
                                    self.event_info,
                                    map_info,
                                    self.pointer_size,
                                    in_type,
                                    out_type,
                                    prop_length,
                                    &self.user_data[*user_data_index as usize..],
                                    &mut buffer_size,
                                    buffer,
                                    &mut userdataconsumed,
                                )
                            };
                            if status == ERROR_SUCCESS.0 {
                                unsafe { prop_buffer.set_len((buffer_size / 2) as usize) };
                                *user_data_index += userdataconsumed;
                                break;
                            }
                            if status == ERROR_INSUFFICIENT_BUFFER.0 {
                                prop_buffer.resize((buffer_size / 2) as usize, 0);
                                continue;
                            }
                            if status == ERROR_EVT_INVALID_EVENT_DATA.0 && map_info.is_some() {
                                map_info = None;
                                continue;
                            }
                            error!("Failed to TdhFormatProperty: {}", status);
                            return Err(Error::from(E_FAIL));
                        }
                    }
    
                    let s = U16CStr::from_slice(prop_buffer.as_slice())
                        .unwrap_or_default()
                        .to_string()
                        .unwrap_or_default();
                    properties_array.push(s);
    
                    array_index += 1;
                }
                if is_array {
                    properties_object.insert(property_name, PropertyDecoded::Array(properties_array));
                } else {
                    properties_object.insert(property_name, PropertyDecoded::String(properties_array[0].clone()));
                }
            }
            property_index += 1;
        }
        Ok(properties_object)
    }
}

#[inline]
pub fn is_string_event(flag: u16) -> bool {
    (flag & EVENT_HEADER_FLAG_STRING_ONLY as u16) != 0
}

#[derive(Debug, Serialize)]
pub struct EventRecordDecoded {
    pub provider_id: Guid,
    pub provider_name: String,
    pub level_name: String,
    pub channel_name: String,
    pub keywords_name: String,
    pub event_name: String,
    pub opcode_name: String,
    pub event_message: String,
    pub provider_message: String,
    pub process_id: u32,
    pub thread_id: u32,
    pub timestamp: TimeStamp,
    pub properties: PropertyDecoded
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PropertyDecoded {
    String(String),
    Array(Vec<String>),
    Struct(LinkedHashMap<String, PropertyDecoded>),
}
