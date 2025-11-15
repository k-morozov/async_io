use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use crate::executor::event;

pub(crate) struct Scheduler<F>
where
    F: Future + Send + 'static,
{
    inner: Arc<SchedulerImpl<F>>,
    events_handle: Cell<Option<JoinHandle<()>>>,
}

impl<F> Scheduler<F>
where
    F: Future + Send + 'static,
{
    fn new() -> Self {
        Self {
            inner: SchedulerImpl::new(),
            events_handle: Cell::new(None),
        }
    }

    pub fn activate(&self) {
        let inner = self.inner.clone();
        let events_handle: JoinHandle<()> = std::thread::Builder::new()
            .name("events_proccessor".to_string())
            .spawn(move || {
                inner.loop_proccess();
            })
            .unwrap();

        self.events_handle.set(Some(events_handle));
    }

    pub fn deactivate(&self) {
        self.inner.deactivate();
    }

    pub fn push_event(&self, event: event::Event<F>) {}
}

impl<F: Future + Send + 'static> Drop for Scheduler<F> {
    fn drop(&mut self) {
        log::debug!("call drop");

        let h = self.events_handle.take().unwrap();
        h.join().unwrap();
    }
}

struct SchedulerImpl<F>
where
    F: Future + Send + 'static,
{
    in_progress: Arc<(Mutex<VecDeque<event::Event<F>>>, Condvar)>,
    shutdown: Mutex<bool>,
}

unsafe impl<F> Sync for SchedulerImpl<F> where F: Future + Send + 'static {}

impl<F> SchedulerImpl<F>
where
    F: Future + Send + 'static,
{
    fn new() -> Arc<Self> {
        Arc::new(Self {
            in_progress: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            shutdown: Mutex::new(false),
        })
    }

    fn deactivate(&self) {
        let mut g = self.shutdown.lock().unwrap();
        (*g) = true;
    }

    fn is_shutdown(&self) -> bool {
        *self.shutdown.lock().unwrap()
    }

    fn loop_proccess(&self) {
        loop {
            if self.is_shutdown() {
                log::debug!("thread was shutdowned, finish.");
                break;
            }

            let (lock, cvar) = &*self.in_progress;
            let guard = lock.lock().unwrap();
            let mut guard = cvar.wait_while(guard, |q| q.is_empty()).unwrap();

            let event = guard.pop_front();

            drop(guard);

            match event {
                Some(mut event) => {
                    event.run();
                }
                None => {
                    log::error!("None in queue")
                }
            }
        }
    }

    pub fn push_event(&self, event: event::Event<F>) {
        if self.is_shutdown() {
            log::debug!("failed, thread was shutdowned");
            return;
        }

        let (q, cvar) = &*self.in_progress;
        let mut guard = q.lock().unwrap();
        guard.push_back(event);
        cvar.notify_one();
    }
}
