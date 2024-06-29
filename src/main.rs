#![feature(sync_unsafe_cell, btree_cursors, map_try_insert)]
//#![windows_subsystem = "windows"]

use event_record_model::Columns;
use i_slint_backend_winit::WinitWindowAccessor;
use linked_hash_map::LinkedHashMap;
use slint::{
    Model, ModelRc, PhysicalPosition, SharedString, StandardListViewItem, TableColumn, VecModel,
};
use std::{cell::SyncUnsafeCell, fs::create_dir_all, path::Path, rc::Rc, sync::{Arc, Weak}};
use strum::VariantArray;
use tracing::{error, info, warn};

use crate::event_record_model::EventRecordModel;

mod delay_notify;
mod event_list;
mod event_list_model;
mod event_record_model;
mod event_trace;
mod filter;
mod pdb;
mod process_modules;
mod third_extend;
mod utils;

slint::include_modules!();

fn main() {
    let file_appender = tracing_appender::rolling::never("./logs", "logs.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    let app = App::new().unwrap();
    let window = app.window();
    if let Some(current_monitor_size) = window
        .with_winit_window(|wininit_windows| {
            wininit_windows
                .current_monitor()
                .map(|current_monitor| current_monitor.size())
        })
        .unwrap_or(None)
    {
        let width = current_monitor_size.width as f32 * 0.6;
        let height = current_monitor_size.height as f32 * 0.6;
        let x = (current_monitor_size.width - width as u32) / 2;
        let y = (current_monitor_size.height - height as u32) / 2;
        app.set_initial_size((height, width));
        window.set_position(PhysicalPosition::new(x as i32, y as i32));
    } else {
        let width = 800.0;
        let height = 600.0;
        app.set_initial_size((height, width));
    }

    let event_list_arc =
        Arc::new(event_list::EventList::<event_record_model::EventRecordModel>::new());
    let event_list_arc_1 = event_list_arc.clone();

    let event_list_model_rc = Rc::new(event_list_model::ListModel::new(event_list_arc));
    let event_list_model_rc_1 = event_list_model_rc.clone();
    let event_list_model_rc_2 = event_list_model_rc.clone();
    let event_list_model_rc_3 = event_list_model_rc.clone();
    let event_list_model_rc_4 = event_list_model_rc.clone();

    let row_data: ModelRc<ModelRc<StandardListViewItem>> = ModelRc::from(event_list_model_rc);
    let column_names_rc = Rc::new(VecModel::default());
    for column in event_record_model::Columns::VARIANTS {
        let mut table_column = TableColumn::default();
        table_column.title = SharedString::from(column.as_ref());
        table_column.width = match column {
            Columns::Datetime => 180.0,
            Columns::ProcessName => 150.0,
            Columns::ProcessId => 110.0,
            Columns::ThreadId => 100.0,
            Columns::EventName => 120.0,
            Columns::OpcodeName => 140.0,
            Columns::Properties => 200.0,
        };
        column_names_rc.push(table_column);
    }
    app.global::<EventsViewData>()
        .set_column_names(ModelRc::from(column_names_rc));
    app.global::<EventsViewData>().set_row_data(row_data);
    app.global::<EventsViewData>()
        .on_row_data_detail(move |index_row| {
            let mut ret = SharedString::from("");
            if let Some(row) = event_list_model_rc_1.row_data_detail(index_row as usize) {
                if let Some(row_item) = row
                    .value
                    .as_any()
                    .downcast_ref::<event_record_model::EventRecordModel>()
                {
                    ret = row_item.data_detail().unwrap_or_default();
                }
            }
            ret
        });
    app.global::<EventsViewData>()
        .on_stack_walk(move |index_row| {
            if let Some(row) = event_list_model_rc_2.row_data_detail(index_row as usize) {
                if let Some(row_item) = row
                    .value
                    .as_any()
                    .downcast_ref::<event_record_model::EventRecordModel>()
                {
                    return row_item.stack_walk();
                }
            }
            (StackWalkInfo::default(), StackWalkInfo::default())
        });
    app.global::<EventsViewData>().on_row_find(move |text| {
        if text.is_empty() {
            return (SharedString::default(), ModelRc::default(), true);
        }
        let r = filter::ExpressionForOne::parse(text.as_str());
        let fe = match r {
            Ok(fe) => fe,
            Err(e) => return (SharedString::from(e.to_string()), ModelRc::default(), false),
        };
        match event_list_model_rc_3.row_find(&fe) {
            Ok(vec) => (
                SharedString::default(),
                ModelRc::new(VecModel::from(vec)),
                true,
            ),
            Err(e) => (SharedString::from(e.to_string()), ModelRc::default(), false),
        }
    });

    let mut event_descs = vec![];
    for major in event_trace::EVENTS_DESC.iter() {
        let mut minors: Vec<(bool, SharedString)> = vec![];
        for minor in major.minors {
            minors.push((false, minor.name.into()));
        }
        event_descs.push(EventDesc {
            is_config: major.configurable,
            enable: false,
            name: if let Some(name) = major.major.display_name {
                name.into()
            } else {
                major.major.name.into()
            },
            minors: ModelRc::from(minors.as_slice()),
        });
    }
    app.global::<EnablesData>()
        .set_event_descs(ModelRc::from(event_descs.as_slice()));
    app.global::<EnablesData>()
        .on_toggled_major(|index, checked| {
            event_trace::Controller::set_config_enables(index as usize, None, checked);
        });
    app.global::<EnablesData>()
        .on_toggled_minor(|index_major, index_minor, checked| {
            event_trace::Controller::set_config_enables(
                index_major as usize,
                Some(index_minor as usize),
                checked,
            );
        });
    let event_descs_1 = event_descs.clone();
    app.global::<EnablesData>().on_row_find(move |event_name| {
        if event_name.is_empty() {
            return (SharedString::default(), ModelRc::default(), true);
        }
        let mut vec = vec![];
        for (index, event_desc) in event_descs_1.iter().enumerate() {
            if event_desc
                .name
                .to_ascii_lowercase()
                .contains(event_name.to_ascii_lowercase().as_str())
            {
                vec.push(index as i32);
            }
        }
        (
            SharedString::default(),
            ModelRc::new(VecModel::from(vec)),
            true,
        )
    });

    app.on_set_filter_expression_for_one(|text| {
        if text.is_empty() {
            filter::filter_expression_for_one_set(None);
            return (SharedString::new(), true);
        }
        match filter::ExpressionForOne::parse(text.as_str()) {
            Err(e) => (SharedString::from(e.to_string()), false),
            Ok(ok) => {
                filter::filter_expression_for_one_set(Some(ok));
                (SharedString::new(), true)
            }
        }
    });
    app.on_set_filter_expression_for_pair(|text| {
        if text.is_empty() {
            filter::filter_expression_for_pair_set(vec![]);
            return (SharedString::new(), true);
        }
        match filter::ExpressionForPair::parse(text.as_str()) {
            Err(e) => (SharedString::from(e.to_string()), false),
            Ok(ok) => {
                filter::filter_expression_for_pair_set(ok);
                (SharedString::new(), true)
            }
        }
    });

    match utils::get_exe_dir() {
        Err(e) => warn!("{e}"),
        Ok(path) => {
            let s = format!("{path}\\pdb");
            let dir = Path::new(s.as_str());
            if let Err(e) = create_dir_all(dir) {
                error!("{e}");
            } else {
                pdb::pdb_path_set(s.as_str());
                app.set_pdb_directory(SharedString::from(s.as_str()))
            }
        }
    }
    app.on_edit_pdb_directory(|path| {
        let dir = Path::new(path.as_str());
        if !dir.exists() {
            return (SharedString::from("The path is not exist"), false);
        }
        if !dir.is_dir() {
            return (SharedString::from("The path is not directory"), false);
        }
        pdb::pdb_path_set(path.as_str());
        (SharedString::new(), true)
    });

    app.on_clear(move || {
        event_list_model_rc_4.clear();
    });

    let app_weak = app.as_weak();
    app.on_trace_start(move || {
        let app_weak_1 = app_weak.clone();
        let event_list_arc_1 = event_list_arc_1.clone();
        let mut stack_walk_map = SyncUnsafeCell::new(LinkedHashMap::<
            (u32, i64),
            Option<Weak<event_list::Node<EventRecordModel>>>,
        >::with_capacity(50));
        let mut stack_walk_delay_remove_map = SyncUnsafeCell::new(LinkedHashMap::<
            (u32, i64),
            Option<Weak<event_list::Node<EventRecordModel>>>,
        >::with_capacity(50));
        let mut delay_notify = Box::new(delay_notify::DelayNotify::new(100, 200));
        delay_notify.init(app_weak_1.clone());
        process_modules::init(&vec![]);
        let result = event_trace::Controller::start(
            move |mut event_record, stack_walk, is_selected| {
                let process_id = event_record.process_id;
                let thread_id = event_record.thread_id;
                let timestamp = event_record.timestamp.0;

                if let Some(mut sw) = stack_walk {
                    if let Some(some_row) = stack_walk_map
                        .get_mut()
                        .remove(&(sw.stack_thread, sw.event_timestamp))
                    {
                        if let Some(ref weak) = some_row {
                            if let Some(arc_node) = weak.upgrade() {
                                process_modules::convert_to_module_offset(
                                    sw.stack_process,
                                    sw.stacks.as_mut_slice(),
                                );
                                let erm = arc_node
                                    .value
                                    .as_any()
                                    .downcast_ref::<event_record_model::EventRecordModel>()
                                    .unwrap();
                                erm.set_stack_walk(sw.clone());
                            }
                        }
                        stack_walk_delay_remove_map
                            .get_mut()
                            .insert((sw.stack_thread, sw.event_timestamp), some_row);
                    } else {
                        if let Some(some_row) = stack_walk_delay_remove_map
                            .get_mut()
                            .remove(&(sw.stack_thread, sw.event_timestamp))
                        {
                            if let Some(ref weak) = some_row {
                                if let Some(arc_node) = weak.upgrade() {
                                    process_modules::convert_to_module_offset(
                                        sw.stack_process,
                                        sw.stacks.as_mut_slice(),
                                    );
                                    let erm = arc_node
                                        .value
                                        .as_any()
                                        .downcast_ref::<event_record_model::EventRecordModel>()
                                        .unwrap();
                                    erm.set_stack_walk_2(sw);
                                }
                            }
                        } else {
                            error!(
                                "Can't find event for the stack walk: {}:{}:{timestamp}  {}:{}:{} {:?}",
                                process_id as i32,
                                thread_id as i32,
                                sw.stack_process,
                                sw.stack_thread as i32,
                                sw.event_timestamp,
                                sw.stacks
                            );
                        }
                    }

                    // clear stack_walk_delay_remove_map. because insert a item when is stack walk event. so keep the map len
                    let map = stack_walk_delay_remove_map.get_mut();
                    map_pop_front(map, timestamp, 10, 10, true);
                    return;
                } else {
                    // clear stack_walk_map. because insert a item when is a non stack walk event. so keep the map len
                    let map = stack_walk_map.get_mut();
                    map_pop_front(map, timestamp, 10, 30, false);
                }

                fn map_pop_front(map: &mut LinkedHashMap<(u32, i64), Option<Weak<event_list::Node<EventRecordModel>>>>, current_timestamp: i64, count: usize, num_seconds: i64, is_delay_remove_map: bool) {
                    let count = if count < map.len() { count } else {map.len() };
                    for _index in 0..count {
                        let is_pop = if let Some((key, _value)) = map.front() {
                            let dt_prev = utils::TimeStamp(key.1).to_datetime_local();
                            let duration = utils::TimeStamp(current_timestamp).to_datetime_local() - dt_prev;
                            if duration.num_seconds() > num_seconds {
                                true
                            } else {
                                break;
                            }
                        } else {
                            break;
                        };

                        if is_pop {
                            let (key, value) = map.pop_front().unwrap();
                            let dt_prev = utils::TimeStamp(key.1).to_datetime_local();
                            let message: String = if let Some(weak) = value {
                                if let Some(arc_node) = weak.upgrade() {
                                    let erm = arc_node
                                    .value
                                    .as_any()
                                    .downcast_ref::<event_record_model::EventRecordModel>()
                                    .unwrap();
                                    format!("{:?}", *erm.array)
                                } else {
                                    format!("had been drop")
                                }
                            } else {
                                format!("Not push to list")
                            };

                            if !is_delay_remove_map {
                                warn!("No stack walk for the event: process: {} {}  {message}.", key.0, dt_prev.to_string())
                            }
                        }
                    }
                }

                process_modules::handle_event_for_module(&mut event_record);
                if !is_selected {
                    return;
                }

                let er = event_record_model::EventRecordModel::new(
                    event_record,
                    process_modules::get_process_path_by_id(process_id),
                );
                let is_matched = match filter::filter_for_one(
                    |path, value| er.find_by_path_value(path, value),
                    |value| er.find_by_value(value),
                ) {
                    Err(e) => {
                        error!("Failed to filter: {e}");
                        return;
                    }
                    Ok(is_matched) => is_matched,
                };

                let row_arc = Arc::new(event_list::Node::new(er));
                let mut is_push_to_list = false;
                let mut notify: Option<delay_notify::Notify> = None;
                if is_matched {
                    match filter::filter_for_pair(&row_arc) {
                        Err(e) => {
                            error!("Failed to filter: {e}");
                            is_push_to_list = true;
                        }
                        Ok(ok) => {
                            if let Some(node) = ok {
                                event_list_arc_1.remove(node);
                                notify = Some(delay_notify::Notify::Remove);
                            } else {
                                is_push_to_list = true;
                            }
                        }
                    }
                }

                if is_push_to_list {
                    stack_walk_map
                        .get_mut()
                        .insert((thread_id, timestamp), Some(Arc::downgrade(&row_arc)));
                    let index = event_list_arc_1.push(row_arc);
                    notify = Some(delay_notify::Notify::Push(index, 1));
                } else {
                    stack_walk_map
                        .get_mut()
                        .insert((thread_id, timestamp), None);
                }

                if let Some(notify) = notify {
                    delay_notify.notify(app_weak_1.clone(), notify);
                }
            },
            |ret| {
                info!("{:?}", ret);
            },
        );
        if let Err(e) = result {
            error!("{}", e);
            (SharedString::from(e.to_string()), false)
        } else {
            (SharedString::from(""), true)
        }
    });
    app.on_trace_stop(|| {
        let r = event_trace::Controller::stop(None);
        info!("end: {:?}", r);
    });

    app.run().unwrap();

    info!("end");
}
