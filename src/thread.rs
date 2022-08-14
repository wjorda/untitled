use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;
use log::{debug, info};
use common::Config;

pub fn process(rx: Receiver<u8>, config: Config) {
    debug!("Thread started!");
    let mut counter = 0;
    loop {
        if let Ok(signal) = rx.try_recv() {
            debug!("Signal {} received, killing thread", signal);
            return;
        }
        info!("Thread has been running for {} seconds!", counter);
        counter += 1;
        sleep(Duration::from_secs(1))
    }
}