pub mod waker;

use std::cell::{Cell, RefCell};
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
use std::time::Duration;

use crate::reactor::Reactor;

pub struct Executor<F: Future + Send + 'static> {
    reactor: Arc<Reactor>,
    table: Arc<Mutex<HashMap<waker::TWakerID, Cell<Mutex<Pin<Box<F>>>>>>>,
    task_id_to_waker: Arc<Mutex<HashMap<waker::TWakerID, Waker>>>,
    events_sender: mpsc::Sender<waker::TWakerID>,
    events_receiver: Option<mpsc::Receiver<waker::TWakerID>>,

    id: Mutex<waker::TWakerID>,

    reactor_handle: Cell<Option<JoinHandle<()>>>,
    events_handle: Cell<Option<JoinHandle<()>>>,

    one_shot: Option<mpsc::Sender<F::Output>>,
}

impl<F> Executor<F>
where 
    F: Future + Send + 'static,
    F::Output: Send {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let reactor = Arc::new(Reactor::new());
        let reactor_to_handle = reactor.clone();

        let reactor_handle = std::thread::spawn(move || {
                reactor_to_handle.run_loop();
        });

        Self {
            reactor,
            table: Arc::new(Mutex::new(HashMap::new())),
            task_id_to_waker: Arc::new(Mutex::new(HashMap::new())),
            events_sender: tx,
            events_receiver: Some(rx),
            id: Mutex::new(1),
            reactor_handle: Cell::new(Some(reactor_handle)),
            events_handle: Cell::new(None),
            one_shot: None,
        }
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on(&mut self, future: F) -> F::Output {
        let task_id = self.generate_id();

        let (rx, tx) = channel();

        self.one_shot = Some(rx);

        self.table.lock().unwrap().insert(task_id, Cell::new(Mutex::new(Box::pin(future))));

        self.task_id_to_waker.lock().unwrap().insert(
            task_id, 
            waker::make(task_id, self.events_sender.clone())
        );

        let wk = self.task_id_to_waker.lock().unwrap().get(&task_id).unwrap().clone();
        let mut ctx = Context::from_waker(&wk);

        {
            let mut g_table = self.table.lock().unwrap();
            let mut g_future = g_table.get_mut(&task_id).unwrap().get_mut().lock().unwrap();
            if let Poll::Ready(output) = g_future.as_mut().poll(&mut ctx) {
                log::debug!("Future is ready without select call, block_on is completed.");
                return output;
            }
        }
        
        loop {
            match tx.try_recv() {
                Ok(output) => {
                    log::debug!("block_on completed");
                    return output;
                },
                Err(er) => {
                    match er {
                            mpsc::TryRecvError::Empty => {
                                let timeout = Duration::from_secs(1);
                                log::debug!("block_on: {er}, sleep {} sec", timeout.as_secs());
                                std::thread::sleep(timeout);
                            },
                            mpsc::TryRecvError::Disconnected => {
                                log::error!("block_on: {er}");
                                panic!("block_on: broken channel");
                            },
                        }
                },
            }
        }
        /* `<F as Future>::Output` value */
    }

    fn generate_id(&self) -> waker::TWakerID {
        let g = self.id.lock().unwrap();
        (*g).wrapping_add(1)
    }

    pub fn run_events_loop(&mut self) {
        let rx = self.events_receiver.take().unwrap();
        let tx = self.one_shot.take();

        let table = self.table.clone();
        let task_id_to_waker = self.task_id_to_waker.clone();

        let h = std::thread::spawn(move || {
            loop {       
                log::debug!("run_events_loop"); 
                match rx.try_recv() {
                    Ok(task_id) => {
                        log::debug!("Found ready event with task_id {task_id}.");

                        let mut g_table = table.lock().unwrap();
                        let mut g_future = g_table.get_mut(&task_id).unwrap().get_mut().lock().unwrap();
                        // let future = table.lock().unwrap().get_mut(&task_id);

                        // if let None = future {
                        //     log::warn!("Empty future for task_id {}", task_id);
                        //     continue;
                        // }

                        let wk = task_id_to_waker.lock().unwrap().get(&task_id).unwrap().clone();
                        let mut ctx = Context::from_waker(&wk);
                        
                        match g_future.as_mut().poll(&mut ctx) {
                            Poll::Ready(output) => {
                                log::debug!("Future is ready, block_on is completed.");
                                
                                table.lock().unwrap().remove(&task_id);
                                task_id_to_waker.lock().unwrap().remove(&task_id);

                                tx.unwrap().send(output);
                                return ();
                                // return output;
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

        });

        self.events_handle.set(Some(h));
    }
}

impl<F: Future + Send + 'static> Drop for Executor<F> {
    fn drop(&mut self) {
        self.reactor.set_shutdown();
        let h = self.reactor_handle
            .replace(None)
            .expect("created in new");
        h.join().unwrap();

        let h = self.events_handle
            .replace(None)
            .expect("created in new");
        h.join().unwrap();
    }
}
