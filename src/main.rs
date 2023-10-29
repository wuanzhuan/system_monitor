use tracing::{error, info};
use tracing_subscriber;

mod event_trace;
mod third_extend;
mod utils;

slint::include_modules!();

fn main() {
    let file_appender = tracing_appender::rolling::never("./target/debug/logs", "prefix.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_writer(non_blocking)
    .with_ansi(false)
    .init();

    let app = App::new().unwrap();
    let mut event_descs = vec![];
    for major in event_trace::EVENTS_DESC.iter() {
        let mut minors: Vec<slint::SharedString> = vec![];
        for minor in major.minors {
            minors.push(minor.name.into());
        }
        event_descs.push(EventDesc{name: major.major.name.into(), minors: slint::ModelRc::from(minors.as_slice())});
    }
    app.global::<EnablesData>().set_event_descs(slint::ModelRc::from(event_descs.as_slice()));
    app.global::<EnablesData>().on_toggled_major(|index, checked| {
        event_trace::Controller::set_config_enables(index as usize, None, checked);
    });
    app.global::<EnablesData>().on_toggled_minor(|index_major, index_minor, checked| {
        event_trace::Controller::set_config_enables(index_major as usize, Some(index_minor as usize), checked);
    });
    app.on_start(|| {
        let result = event_trace::Controller::start(|event_record|{
            info!("{:?}", event_record);
        }, |ret| {
            info!("{:?}", ret);
        });
        if let Err(e) = result {
            error!("{}", e);
            return;
        }
    });
    app.on_stop(|| {
        let x = event_trace::Controller::stop();
        info!("end: {:?}", x);
    });

    app.run().unwrap();

    info!("end");

}
