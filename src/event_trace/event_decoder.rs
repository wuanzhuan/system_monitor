use crate::third_extend::strings::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::System::Diagnostics::Etw::*,
    Win32::System::SystemInformation::*,
};
use tracing::{error, warn, info};
use std::{mem, slice};
use widestring::*;



pub struct EventRecordDecoded {
    provider_id: GUID,
    provider_name: String,
    level_name: String,
    channel_name: String,
    keywords_name: String,
    event_name: String,
    opcode_name: String,
    event_message: String,
    provider_message: String,
    process_id: String,
    thread_id: String,
}

#[inline]
pub fn is_string_event(flag: u16) -> bool {
    (flag & EVENT_HEADER_FLAG_STRING_ONLY as u16) != 0
}

pub fn properties(
    event_record: &EVENT_RECORD,
    event_info: &TRACE_EVENT_INFO,
    event_info_slice: &[u8],
    property_info_array: &[EVENT_PROPERTY_INFO],
    properties_array_begin: u16,
    properties_array_end: u16,
    user_data: &[u8],
    user_data_begin: usize,
    int_values: &mut [u16],
) {
    let mut user_data_index = user_data_begin;
    let mut index = properties_array_begin;
    while index < properties_array_end {
        let property_info = &property_info_array[index as usize];

        let property_name = u16cstr_from_bytes_truncate_offset(event_info_slice, property_info.NameOffset).unwrap_or_default();

        // If this property is a scalar integer, remember the value in case it
        // is needed for a subsequent property's length or count.
        if 0 == (property_info.Flags.0 & (PropertyStruct.0 | PropertyParamCount.0))
            && unsafe { property_info.Anonymous2.count } == 1
        {
            let in_type = unsafe { property_info.Anonymous1.nonStructType.InType } as i32;
            if in_type == TDH_INTYPE_INT8.0 || in_type == TDH_INTYPE_UINT8.0 {
                if user_data.len() - index as usize >= 1 {
                    int_values[index as usize] = u8::from_ne_bytes(user_data[user_data_index..user_data_index + 1].try_into().unwrap()) as u16;
                }
            } else if in_type == TDH_INTYPE_INT16.0 || in_type == TDH_INTYPE_UINT16.0 {
                if user_data.len() - index as usize >= 2 {
                    int_values[index as usize] = u16::from_ne_bytes(user_data[user_data_index..user_data_index + 2].try_into().unwrap());
                }
            } else if in_type == TDH_INTYPE_INT32.0
                || in_type == TDH_INTYPE_UINT32.0
                || in_type == TDH_INTYPE_HEXINT32.0
            {
                if user_data.len() - index as usize >= 4 {
                    let v = u32::from_ne_bytes(user_data[user_data_index..user_data_index + 4].try_into().unwrap());
                    int_values[index as usize] = if v > 0xffff { 0xffff } else { v as u16 };
                }
            }
        }

        let in_type = unsafe { property_info.Anonymous1.nonStructType.InType };
        let out_type = unsafe { property_info.Anonymous1.nonStructType.OutType };
        let length = unsafe { property_info.Anonymous3.length };
        let prop_length = if out_type == TDH_OUTTYPE_IPV6.0 as u16
            && in_type == TDH_INTYPE_BINARY.0 as u16
            && length == 0
            && (property_info.Flags.0 & (PropertyParamLength.0 | PropertyParamFixedLength.0)) != 0
        {
            16 // special case for incorrectly-defined IPV6 addresses
        } else if (property_info.Flags.0 & PropertyParamLength.0) != 0 {
            int_values[unsafe { property_info.Anonymous3.lengthPropertyIndex } as usize] // Look up the value of a previous property
        } else {
            length
        };

        let (array_count, is_array) = if (property_info.Flags.0 & PropertyParamCount.0) != 0 {
            let count_property_index = unsafe { property_info.Anonymous2.countPropertyIndex };
            if count_property_index >= index as u16 {
                error!("invalid count_property_index: {} index: {}", count_property_index, index);
                return;
            }
            (int_values[count_property_index as usize], true)  // Look up the value of a previous property
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

         // Treat non-array properties as arrays with one element.
         let mut array_index = 0;
         while array_index != array_count && user_data_index < user_data.len() {
            if (property_info.Flags.0 & PropertyStruct.0) != 0 {
                // If this property is a struct, recurse and print the child
                // properties.
                let struct_start_index = unsafe { property_info.Anonymous1.structType.StructStartIndex };
                let num_of_struct_members = unsafe { property_info.Anonymous1.structType.NumOfStructMembers };

                properties(
                    event_record,
                    event_info,
                    event_info_slice,
                    property_info_array,
                    struct_start_index,
                    struct_start_index + num_of_struct_members,
                    user_data,
                    user_data_index,
                    int_values,
                );
                continue;
            }
            // If the property has an associated map (i.e. an enumerated type),
            // try to look up the map data. (If this is an array, we only need
            // to do the lookup on the first iteration.)
            let map_name_offset = unsafe{ property_info.Anonymous1.nonStructType.MapNameOffset };
            let mut map_info: Option<*const EVENT_MAP_INFO> = None;
            if map_name_offset != 0 && array_index == 0 {
                if in_type == TDH_INTYPE_UINT8.0 as u16 || in_type == TDH_INTYPE_UINT16.0 as u16 || in_type == TDH_INTYPE_UINT32.0 as u16 || in_type == TDH_INTYPE_HEXINT32.0 as u16 {
                    let map_name = u16cstr_from_bytes_truncate_offset(event_info_slice, map_name_offset).unwrap_or_default();
                    let mut buffer_size = 1024;
                    loop {
                        let _map_info: &mut EVENT_MAP_INFO = unsafe { mem::transmute(vec![0u8; buffer_size as usize].as_ptr()) };
                        map_info = Some(_map_info);
                        let status = unsafe { TdhGetEventMapInformation(event_record, map_name.as_pcwstr(), Some(_map_info), &mut buffer_size)};
                        if status == ERROR_SUCCESS.0 {
                            break;
                        }
                        if status == ERROR_INSUFFICIENT_BUFFER.0 {
                            continue;
                        }
                        error!("Failed to TdhGetEventMapInformation: {}", status);
                        break;
                    };
                }
            };

            let mut prop_buffer = Vec::<u16>::new();
            let mut status = ERROR_SUCCESS.0;

            if 0 == prop_length && in_type == TDH_INTYPE_NULL.0 as u16
            {
                // TdhFormatProperty doesn't handle INTYPE_NULL.
                prop_buffer.push(0);
            } else if 0 == prop_length 
                      && 0 != (property_info.Flags.0 & (PropertyParamLength.0 | PropertyParamFixedLength.0)) 
                      && (in_type == TDH_INTYPE_UNICODESTRING.0 as u16 || in_type == TDH_INTYPE_ANSISTRING.0 as u16) {
                // TdhFormatProperty doesn't handle zero-length counted strings.
                prop_buffer.push(0);
            }else {
                let pointer_size = if (event_record.EventHeader.Flags as u32 & EVENT_HEADER_FLAG_32_BIT_HEADER) != 0 {
                    4u32
                } else if (event_record.EventHeader.Flags as u32 & EVENT_HEADER_FLAG_64_BIT_HEADER) != 0 {
                    8u32
                } else {
                    mem::size_of::<*const u8>() as u32
                };
                let mut buffer_size = 1024u32;
                prop_buffer.resize((buffer_size / 2) as usize, 0);
                let mut userdataconsumed = 0u16;
                loop {
                    let buffer = PWSTR::from_raw(prop_buffer.as_mut_ptr());
                    status = unsafe {
                        TdhFormatProperty(event_info, map_info, 
                            pointer_size, 
                            in_type, if out_type == TDH_OUTTYPE_NOPRINT.0 as u16 {TDH_OUTTYPE_NULL.0 as u16} else {out_type}, prop_length,
                            &user_data[user_data_index..],
                            &mut buffer_size, buffer,
                            &mut userdataconsumed
                        )
                    };
                    if status == ERROR_SUCCESS.0 {
                        unsafe{ prop_buffer.set_len((buffer_size / 2) as usize) };
                        user_data_index += userdataconsumed as usize;
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
                    break;
                }
            }
            if status != ERROR_SUCCESS.0 {
                break;
            }

            let s = U16CStr::from_slice(prop_buffer.as_slice()).unwrap_or_default();
            info!("property: {:?}-{}: {:#?}", property_name, array_index, s);

            array_index += 1;
         }

        index += 1;
    }
}
