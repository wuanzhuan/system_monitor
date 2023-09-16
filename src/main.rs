use std::{thread, time};
use tracing::{error, info};
use tracing_subscriber;

mod event_trace;
mod third_extend;
mod utils;

fn main() {
    tracing_subscriber::fmt::init();

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
