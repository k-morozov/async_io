use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub struct EventHandler<T> {
    result: Box<Mutex<Receiver<T>>>,
}

impl<T> EventHandler<T> {
    pub fn new(rx: Receiver<T>) -> Arc<EventHandler<T>> {
        Arc::new(Self {
            result: Box::new(Mutex::new(rx)),
        })
    }

    pub fn wait_result(&self) -> T {
        loop {
            let guard = self.result.lock().unwrap();
            match guard.try_recv() {
                Ok(output) => {
                    log::debug!("Handler obtained the output");
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
