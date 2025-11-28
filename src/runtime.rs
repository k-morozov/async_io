mod event;
mod event_handler;
mod scheduler;
mod waker;

use std::cell::Cell;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use crate::reactor::Reactor;

use self::event::TEventID;
use self::event_handler::EventHandler;
use self::scheduler::Scheduler;

pub struct Runtime<T: Send + 'static> {
    reactor: Arc<Reactor>,
    reactor_handle: Mutex<Option<JoinHandle<()>>>,

    id: AtomicU64,

    scheduler: Arc<Scheduler<T>>,
}

unsafe impl<T: Send + 'static> Sync for Runtime<T> {}

impl<T: Send + 'static> Runtime<T> {
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
            reactor_handle: Mutex::new(Some(reactor_handle)),
            id: AtomicU64::new(1),
            scheduler: Arc::new(Scheduler::new()),
        }
    }

    pub fn start(&self) {
        self.scheduler.activate();
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on<F: Future<Output = T> + Send + 'static>(&self, future: F) -> F::Output {
        let event_id = self.generate_id();

        let scheduler = self.scheduler.clone();
        let resume = move |event_id: TEventID| {
            log::debug!("call resume for waker with event_id={event_id}.");
            scheduler.resume_event(event_id);
        };

        let waker = waker::make(event_id, Box::new(resume));
        let ev = event::Event::new(
            event_id,
            future,
            waker,
            event::ReschedulerPolicy::InProgress,
        );

        let handler = self.scheduler.push_event(ev);

        handler.wait_result()
    }

    pub fn spawn<F: Future<Output = T> + Send + 'static>(&self, future: F) -> Arc<EventHandler<T>> {
        log::debug!("call spawn");
        let event_id = self.generate_id();

        let scheduler = self.scheduler.clone();
        let resume = move |event_id: TEventID| {
            log::debug!("call resume for waker with event_id={event_id}.");
            scheduler.resume_event(event_id);
        };

        let waker = waker::make(event_id, Box::new(resume));
        let ev = event::Event::new(event_id, future, waker, event::ReschedulerPolicy::Suspend);

        let handler = self.scheduler.push_event(ev);
        handler
    }

    // trait?
    fn generate_id(&self) -> TEventID {
        self.id.fetch_add(1, Ordering::Relaxed)
    }
}

impl<T: Send + 'static> Drop for Runtime<T> {
    fn drop(&mut self) {
        log::debug!("call drop");

        self.reactor.set_shutdown();
        let mut guard = self.reactor_handle.lock().unwrap();

        let h = guard.take().unwrap();
        h.join().unwrap();

        self.scheduler.deactivate();
    }
}
