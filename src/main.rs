#![feature(linked_list_cursors)]


use std::rc::Rc;
use struct_field_names_as_array::FieldNamesAsArray;
use tracing::{error, info};
use slint::{SharedString, ModelRc, StandardListViewItem, TableColumn, Model, LogicalPosition, VecModel};

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

    let columns = Rc::new(VecModel::<TableColumn>::default());
    let mut tc = TableColumn::default();
    tc.title = SharedString::from("序号");
    tc.width = 6.0 * app.get_rem();
    columns.push(tc);
    for item in event_trace::EventRecordDecoded::FIELD_NAMES_AS_ARRAY {
        let mut tc = TableColumn::default();
        tc.title = SharedString::from(item);
        tc.width = (item.chars().count() + 2) as f32 * app.get_rem();
        columns.push(tc);
    }

    app.global::<EventsViewData>().set_columns(ModelRc::from(columns));

    let event_list_rc = Rc::new(event_list_model::ListModel::<ModelRc<StandardListViewItem>>::new());

    let row_data: ModelRc<ModelRc<StandardListViewItem>> = ModelRc::from(event_list_rc);
    app.global::<EventsViewData>().set_row_data(row_data);

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
        let result = event_trace::Controller::start(move |event_record| {
            let r = app_weak.upgrade_in_event_loop(move |app_handle|{
                 if let Some(rows) = app_handle.global::<EventsViewData>().get_row_data().as_any().downcast_ref::<event_list_model::ListModel::<ModelRc<StandardListViewItem>>>() {
                    let er = event_record_model::EventRecordModel::new(rows.row_count(), &event_record);
                    rows.push(ModelRc::new(er));
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
        let x = event_trace::Controller::stop(None);
        info!("end: {:?}", x);
    });

    app.run().unwrap();

    info!("end");

}
