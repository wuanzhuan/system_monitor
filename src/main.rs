
mod event_trace;
use std::{thread, time};

fn main() {
    let arc_context = event_trace::CONTEXT.clone();

    let mut etw_context = arc_context.lock().unwrap();

    if let Err(x) = etw_context.start(){
        println!("{:?}", x);
        return;
    }

    println!("hello");

    let ten_millis = time::Duration::from_secs(5);
    thread::sleep(ten_millis);

    let x = etw_context.stop();

    println!("end: {:?}", x);
}