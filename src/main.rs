#![feature(linked_list_cursors)]


use std::rc::Rc;
use tracing::{error, info};
use slint::{SharedString, ModelRc, StandardListViewItem, Model, LogicalPosition};

mod event_trace;
mod event_list_model;
mod event_record_model;
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
    window.set_position(LogicalPosition::new(1000.0, 500.0));

    let event_list_rc = Rc::new(event_list_model::ListModel::<ModelRc<StandardListViewItem>>::new());
    let event_list_rc_1 = event_list_rc.clone();
    let event_list_rc_2 = event_list_rc.clone();
    
    let row_data: ModelRc<ModelRc<StandardListViewItem>> = ModelRc::from(event_list_rc);
    app.global::<EventsViewData>().set_row_data(row_data);
    app.global::<EventsViewData>().on_row_data_detail(move |index_row| {
        let mut ret = SharedString::from("");
        if let Some(row) = event_list_rc_1.row_data_detail(index_row as usize) {
            if let Some(row_item) = row.as_any().downcast_ref::<event_record_model::EventRecordModel>() {
                ret = row_item.data_detail().unwrap_or_default();
            }
        }
        ret 
    });
    app.global::<EventsViewData>().on_stack_walk(move |index_row| {
        if let Some(row) = event_list_rc_2.row_data_detail(index_row as usize) {
            if let Some(row_item) = row.as_any().downcast_ref::<event_record_model::EventRecordModel>() {
                return row_item.stack_walk();
            }
        }
        StackWalkInfo::default()
    });

    let mut event_descs = vec![];
    for major in event_trace::EVENTS_DESC.iter() {
        let mut minors: Vec<SharedString> = vec![];
        for minor in major.minors {
            minors.push(minor.name.into());
        }
        event_descs.push(EventDesc{name: major.major.name.into(), minors: ModelRc::from(minors.as_slice())});
    }
    app.global::<EnablesData>().set_event_descs(ModelRc::from(event_descs.as_slice()));
    app.global::<EnablesData>().on_toggled_major(|index, checked| {
        event_trace::Controller::set_config_enables(index as usize, None, checked);
    });
    app.global::<EnablesData>().on_toggled_minor(|index_major, index_minor, checked| {
        event_trace::Controller::set_config_enables(index_major as usize, Some(index_minor as usize), checked);
    });
    let app_weak = app.as_weak();
    app.on_start(move || {
        let app_weak = app_weak.clone();
        let result = event_trace::Controller::start(move |event_record, is_stack_walk| {
            app_weak.upgrade_in_event_loop(move |app_handle|{
                let row_data = app_handle.global::<EventsViewData>().get_row_data();
                let rows = row_data.as_any().downcast_ref::<event_list_model::ListModel::<ModelRc<StandardListViewItem>>>().unwrap();
                if !is_stack_walk {
                    let thread_id = event_record.thread_id;
                    let timestamp = event_record.timestamp.0;
                    let er = event_record_model::EventRecordModel::new(event_record);
                    let row_rc = Rc::new(ModelRc::new(er));
                    rows.stack_walk_map.borrow_mut().insert((thread_id, timestamp), row_rc.clone());
                    rows.push(row_rc);
                } else {
                    let sw = event_trace::StackWalk::from_event_record_decoded(&event_record);
                    if let Some(row_rc) = rows.stack_walk_map.borrow_mut().remove(&(sw.stack_thread, sw.event_timestamp)) {
                        let erm = row_rc.as_any().downcast_ref::<event_record_model::EventRecordModel>().unwrap();
                        if !erm.set_stack_walk(sw.clone()) {
                            error!("Stalkwalk event had been set! {}-{}", sw.stack_thread as i32, sw.event_timestamp);
                        }
                    } else {
                        error!("Can't find the stack walk by: {}-{}",  sw.stack_thread as i32, sw.event_timestamp);
                    }
                }
            }).unwrap();

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
