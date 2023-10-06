use std::{thread, time};
use tracing::{error, info};
use tracing_subscriber;

mod event_trace;
mod third_extend;
mod utils;

fn main() {
    let file_appender = tracing_appender::rolling::never("./target/debug/logs", "prefix.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_writer(non_blocking)
    .with_ansi(false)
    .init();

    let result = event_trace::Controller::start(|ret| {
        print!("{:?}", ret);
    });

    if let Err(e) = result {
        error!("{}", e);
        return;
    }

    info!("hello");

    let ten_millis = time::Duration::from_secs(5);
    thread::sleep(ten_millis);

    let x = event_trace::Controller::stop();

    info!("end: {:?}", x);
}
