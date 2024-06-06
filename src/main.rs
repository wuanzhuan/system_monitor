#![feature(sync_unsafe_cell, btree_cursors, map_try_insert)]
//#![windows_subsystem = "windows"]

use event_list::Node;
use i_slint_backend_winit::WinitWindowAccessor;
use linked_hash_map::LinkedHashMap;
use slint::{Model, ModelRc, PhysicalPosition, SharedString, StandardListViewItem, VecModel};
use std::{cell::SyncUnsafeCell, rc::Rc, sync::Arc};
use tracing::{error, info};

use crate::event_record_model::EventRecordModel;

mod delay_notify;
mod event_list;
mod event_list_model;
mod event_record_model;
mod event_trace;
mod filter;
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
        app.set_preferred_size((height, width));
        window.set_position(PhysicalPosition::new(x as i32, y as i32));
    } else {
        let width = 800.0;
        let height = 600.0;
        app.set_preferred_size((height, width));
    }

    let event_list_arc =
        Arc::new(event_list::EventList::<event_record_model::EventRecordModel>::new());
    let event_list_arc_1 = event_list_arc.clone();

    let event_list_model_rc = Rc::new(event_list_model::ListModel::new(event_list_arc));
    let event_list_model_rc_1 = event_list_model_rc.clone();
    let event_list_model_rc_2 = event_list_model_rc.clone();
    let event_list_model_rc_3 = event_list_model_rc.clone();

    let row_data: ModelRc<ModelRc<StandardListViewItem>> = ModelRc::from(event_list_model_rc);
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
            StackWalkInfo::default()
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
        let mut minors: Vec<SharedString> = vec![];
        for minor in major.minors {
            minors.push(minor.name.into());
        }
        event_descs.push(EventDesc {
            name: major.major.name.into(),
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

    let app_weak = app.as_weak();
    app.on_start(move || {
        let app_weak_1 = app_weak.clone();
        let event_list_arc_1 = event_list_arc_1.clone();
        let mut stack_walk_map = SyncUnsafeCell::new(LinkedHashMap::<(u32, i64), Option<Arc<Node<EventRecordModel>>>>::with_capacity(50));
        let mut delay_notify = Box::new(delay_notify::DelayNotify::new(100, 200));
        delay_notify.init(app_weak_1.clone());
        process_modules::init(&vec![]);
        let result = event_trace::Controller::start(move |mut event_record, stack_walk, is_selected | {
            if let Some(mut sw) = stack_walk {
                if let Some(some_row) = stack_walk_map.get_mut().remove(&(sw.stack_thread, sw.event_timestamp)) {
                    if let Some(some_node) = some_row {
                        process_modules::convert_to_module_offset(sw.stack_process, sw.stacks.as_mut_slice());
                        let erm = some_node.value.as_any().downcast_ref::<event_record_model::EventRecordModel>().unwrap();
                        erm.set_stack_walk(sw.clone());
                    }
                } else {
                    let process_id = event_record.process_id as i32;
                    let thread_id = event_record.thread_id as i32;
                    let timestamp = event_record.timestamp.0;
                    error!("Can't find event for the stack walk: {process_id}:{thread_id}:{timestamp}  {}:{}:{} {:?}",  sw.stack_process, sw.stack_thread as i32, sw.event_timestamp, sw.stacks);
                }
            } else {
                process_modules::handle_event_for_module(&mut event_record);

                if is_selected {
                    let thread_id = event_record.thread_id;
                    let timestamp = event_record.timestamp.0;
                    let er = event_record_model::EventRecordModel::new(event_record);

                    let mut is_matched = false;
                    let (filter_expression_for_one, filter_expression_for_pair) = filter::filter_expression_get();
                    if let Some(ref expression_for_one) = *filter_expression_for_one {
                        match filter::ExpressionForOne::evaluate(expression_for_one, |path, value| {
                                er.find_by_path_value(path, value)
                            }, |value| {
                                er.find_by_value(value)
                            }) {
                            Err(e) => {error!("Failed to evaluate: {e}")},
                            Ok(ok) => is_matched = ok
                        }
                    }
                    if is_matched {
                       for expression_for_pair in filter_expression_for_pair.iter() {
                            match expression_for_pair {
                                filter::ExpressionForPair::Handle => {},
                                filter::ExpressionForPair::Memory => {},
                                filter::ExpressionForPair::Custom { event_name, opcode_name_first, opcode_name_second, fields_for_match } => {}
                            }
                        }
                        let row_arc = Arc::new(event_list::Node::new(er));
                        stack_walk_map.get_mut().insert((thread_id, timestamp), Some(row_arc.clone()));
                        let index = event_list_arc_1.push(row_arc);
                        delay_notify.notify(app_weak_1.clone(), index, delay_notify::NotifyType::Push);
                    } else {
                        stack_walk_map.get_mut().insert((thread_id, timestamp), None);
                    }
                }
            }
        }, |ret| {
            info!("{:?}", ret);
        });
        if let Err(e) = result {
            error!("{}", e);
            (SharedString::from(e.to_string()), false)
        } else {
            (SharedString::from(""), true)
        }
    });
    app.on_stop(|| {
        let r = event_trace::Controller::stop(None);
        info!("end: {:?}", r);
    });

    app.run().unwrap();

    info!("end");
}
