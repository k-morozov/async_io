pub mod waker;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::task::Context;
use std::task::Poll;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::reactor::Reactor;

pub struct Executor<F: Future> {
    reactor: Arc<Reactor>,
    table: HashMap<waker::TWakerID, Pin<Box<F>>>,
    events_sender: mpsc::Sender<waker::TWakerID>,
    events_receiver: mpsc::Receiver<waker::TWakerID>,

    id: Mutex<waker::TWakerID>,

    reactor_handle: Cell<Option<JoinHandle<()>>>,
}

impl<F: Future> Executor<F> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let reactor = Arc::new(Reactor::new());
        let reactor_to_handle = reactor.clone();

        let reactor_handle = std::thread::spawn(move || {
            loop {
                reactor_to_handle.poll_once();
                if reactor_to_handle.is_shutdown() {
                    log::debug!("Compliting reactor thread.");
                    break;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Self {
            reactor,
            table: HashMap::new(),
            events_sender: tx,
            events_receiver: rx,
            id: Mutex::new(1),
            reactor_handle: Cell::new(Some(reactor_handle)),
        }
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on(&mut self, future: F) -> F::Output {
        let id = self.generate_id();

        self.table.insert(id, Box::pin(future));

        let waker = waker::make(id, self.events_sender.clone());
        let mut ctx = Context::from_waker(&waker);

        let future = self.table.get_mut(&id).unwrap();
        if let Poll::Ready(output) = future.as_mut().poll(&mut ctx) {
            log::debug!("Future is ready without select call, block_on is completed.");
            return output;
        }

        loop {
            match self.events_receiver.try_recv() {
                Ok(id) => {
                    log::debug!("Found ready event with id {id}.");

                    let future = self.table.get_mut(&id);

                    if let None = future {
                        log::warn!("Empty future for id {}", id);
                        continue;
                    }

                    match future.unwrap().as_mut().poll(&mut ctx) {
                        Poll::Ready(output) => {
                            log::debug!("Future is ready, block_on is completed.");
                            return output;
                        }
                        Poll::Pending => {
                            log::debug!("Future is pending, pool once.");
                        }
                    }
                },
                Err(er) => {
                    match er {
                        mpsc::TryRecvError::Empty => {
                            let timeout = Duration::from_secs(1);
                            log::debug!("{er}, sleep {} sec", timeout.as_secs());
                            std::thread::sleep(timeout);
                        },
                        mpsc::TryRecvError::Disconnected => {
                            log::error!("{er}");
                            panic!("broken channel");
                        },
                    }
                    
                },
            }

        }
    }

    fn generate_id(&self) -> waker::TWakerID {
        let g = self.id.lock().unwrap();
        (*g).wrapping_add(1)
    }
}

impl<F: Future> Drop for Executor<F> {
    fn drop(&mut self) {
        self.reactor.set_shutdown();
        let h = self.reactor_handle
            .replace(None)
            .expect("created in new");
        h.join().unwrap();
    }
}
