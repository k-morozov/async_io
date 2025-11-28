use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use super::event::TEventID;

pub struct EventHandler<T> {
    event_id: TEventID,
    result: Box<Mutex<Receiver<T>>>,
}

unsafe impl<T> Sync for EventHandler<T> {}

impl<T> EventHandler<T> {
    pub fn new(event_id: TEventID, rx: Receiver<T>) -> Arc<EventHandler<T>> {
        Arc::new(Self {
            event_id,
            result: Box::new(Mutex::new(rx)),
        })
    }

    pub fn wait_result(&self) -> T {
        loop {
            let guard = self.result.lock().unwrap();
            match guard.try_recv() {
                Ok(output) => {
                    log::debug!(
                        "Handler obtained the output for event_id={}.",
                        self.event_id
                    );
                    return output;
                }
                Err(er) => match er {
                    std::sync::mpsc::TryRecvError::Empty => {
                        std::thread::yield_now();
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        panic!("unexpected situation");
                    }
                },
            }
        }
    }
}

impl<T> Drop for EventHandler<T> {
    fn drop(&mut self) {
        log::debug!("drop handler for event_id={}", self.event_id);
    }
}
