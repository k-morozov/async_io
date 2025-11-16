mod event;
mod event_handler;
mod scheduler;
mod waker;

use std::cell::Cell;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

use crate::reactor::Reactor;

use self::scheduler::Scheduler;

pub struct Executor<F: Future + Send + 'static> {
    reactor: Arc<Reactor>,
    reactor_handle: Cell<Option<JoinHandle<()>>>,

    id: Mutex<waker::TWakerID>,

    scheduler: Arc<Scheduler<F>>,
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
            .name("reactor".to_string())
            .spawn(move || {
                reactor_to_handle.run_loop();
            })
            .unwrap();

        Self {
            reactor,
            reactor_handle: Cell::new(Some(reactor_handle)),
            id: Mutex::new(1),
            scheduler: Arc::new(Scheduler::new()),
        }
    }

    pub fn start(&self) {
        self.scheduler.activate();
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on(&mut self, future: F) -> F::Output {
        let task_id = self.generate_id();

        let scheduler = self.scheduler.clone();
        let resume = move |id: waker::TWakerID| {
            log::debug!("call resume for waker with id={id}");
            scheduler.resume_event(id);
        };

        let waker = waker::make(task_id, Box::new(resume));
        let ev = event::Event::new(task_id, future, waker);

        let handler = self.scheduler.push_event(ev);

        handler.wait_result()
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
