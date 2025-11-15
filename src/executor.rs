mod event;
mod scheduler;
mod waker;

use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{self, channel};
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread::JoinHandle;

use crate::reactor::Reactor;

pub struct Executor<F: Future + Send + 'static> {
    reactor: Arc<Reactor>,
    table: Arc<Mutex<HashMap<waker::TWakerID, Mutex<Pin<Box<F>>>>>>,
    task_id_to_waker: Arc<Mutex<HashMap<waker::TWakerID, Waker>>>,

    id: Mutex<waker::TWakerID>,

    reactor_handle: Cell<Option<JoinHandle<()>>>,
}

impl<F> Executor<F>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    pub fn new() -> Self {
        let reactor = Arc::new(Reactor::new());
        let reactor_to_handle = reactor.clone();

        let reactor_handle = std::thread::Builder::new()
            .name("reactor_thread".to_string())
            .spawn(move || {
                reactor_to_handle.run_loop();
            })
            .unwrap();

        Self {
            reactor,
            table: Arc::new(Mutex::new(HashMap::new())),
            task_id_to_waker: Arc::new(Mutex::new(HashMap::new())),
            id: Mutex::new(1),
            reactor_handle: Cell::new(Some(reactor_handle)),
        }
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on(&mut self, future: F) -> F::Output {
        let task_id = self.generate_id();

        let (tx, rx) = channel();

        self.table
            .lock()
            .unwrap()
            .insert(task_id, Mutex::new(Box::pin(future)));

        self.task_id_to_waker
            .lock()
            .unwrap()
            .insert(task_id, waker::make(task_id, tx, std::thread::current()));

        let wk = self
            .task_id_to_waker
            .lock()
            .unwrap()
            .get(&task_id)
            .unwrap()
            .clone();

        let mut ctx = Context::from_waker(&wk);

        {
            let mut g_table = self.table.lock().unwrap();
            let g_future = g_table.get_mut(&task_id).unwrap().get_mut().unwrap();
            if let Poll::Ready(output) = g_future.as_mut().poll(&mut ctx) {
                log::debug!("Future is ready without select call, block_on is completed.");
                return output;
            }
        }

        loop {
            log::debug!("loop for block_on");
            match rx.try_recv() {
                Ok(task_id) => {
                    log::debug!("Found ready task woth id {task_id}.");

                    let mut g_table = self.table.lock().unwrap();
                    let g_future = g_table.get_mut(&task_id).unwrap().get_mut().unwrap();

                    let wk = self
                        .task_id_to_waker
                        .lock()
                        .unwrap()
                        .get(&task_id)
                        .unwrap()
                        .clone();
                    let mut ctx = Context::from_waker(&wk);

                    match g_future.as_mut().poll(&mut ctx) {
                        Poll::Ready(output) => {
                            log::debug!("Future is ready, block_on is completed.");

                            g_table.remove(&task_id);
                            self.task_id_to_waker.lock().unwrap().remove(&task_id);

                            return output;
                        }
                        Poll::Pending => {
                            log::debug!("Future is pending, loop once.");
                        }
                    }
                }
                Err(er) => match er {
                    mpsc::TryRecvError::Empty => {
                        // let timeout = Duration::from_secs(1);
                        // std::thread::sleep(timeout);

                        log::debug!("Empty channel - park");
                        std::thread::park();
                    }
                    mpsc::TryRecvError::Disconnected => {
                        log::error!("{er}");
                        panic!("broken channel");
                    }
                },
            }
        }
    }

    // trait?
    fn generate_id(&self) -> waker::TWakerID {
        let g = self.id.lock().unwrap();
        (*g).wrapping_add(1)
    }
}

impl<F: Future + Send + 'static> Drop for Executor<F> {
    fn drop(&mut self) {
        log::debug!("call drop");

        self.reactor.set_shutdown();
        let h = self.reactor_handle.replace(None).expect("created in new");
        h.join().unwrap();
    }
}
