use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use crate::executor::event;

use super::event_handler::EventHandler;
use super::waker;

pub(crate) struct Scheduler<T: Send + 'static> {
    inner: Arc<SchedulerImpl<T>>,
    events_handle: Cell<Option<JoinHandle<()>>>,
}

impl<T: Send + 'static> Scheduler<T> {
    pub(crate) fn new() -> Self {
        Self {
            inner: SchedulerImpl::new(),
            events_handle: Cell::new(None),
        }
    }

    pub(crate) fn activate(&self) {
        let inner = self.inner.clone();
        let events_handle: JoinHandle<()> = std::thread::Builder::new()
            .name("scheduler".to_string())
            .spawn(move || {
                inner.loop_proccess();
            })
            .unwrap();

        self.events_handle.set(Some(events_handle));
    }

    pub(crate) fn deactivate(&self) {
        self.inner.deactivate();
    }

    pub(crate) fn push_event(&self, event: event::Event<T>) -> Arc<EventHandler<T>> {
        self.inner.push_event(event)
    }

    pub(crate) fn resume_event(&self, task_id: waker::TWakerID) {
        self.inner.resume_event(task_id);
    }
}

impl<T: Send + 'static> Drop for Scheduler<T> {
    fn drop(&mut self) {
        log::debug!("call drop");

        self.deactivate();

        match self.events_handle.take() {
            Some(handle) => {
                if let Err(er) = handle.join() {
                    log::error!("Join events_handle finished with error: {:?}", er);
                    panic!("failed join for events_handle");
                }
            }
            None => {
                panic!("Drop failed: None in events_handle");
            }
        }
    }
}

struct SchedulerImpl<T> {
    in_progress: Arc<(Mutex<VecDeque<event::Event<T>>>, Condvar)>,
    suspend_events: Mutex<HashMap<waker::TWakerID, event::Event<T>>>,
    handlers: Mutex<HashMap<waker::TWakerID, Arc<EventHandler<T>>>>,
    shutdown: Mutex<bool>,
}

unsafe impl<T> Sync for SchedulerImpl<T> {}

impl<T> SchedulerImpl<T> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            in_progress: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            suspend_events: Mutex::new(HashMap::new()),
            handlers: Mutex::new(HashMap::new()),
            shutdown: Mutex::new(false),
        })
    }

    fn deactivate(&self) {
        let mut g = self.shutdown.lock().unwrap();
        (*g) = true;

        // notify
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
                Some(mut event) => match event.run() {
                    event::EventStatus::READY(output) => {
                        event.send_to_tx(output);
                        log::debug!("Data was sent to event.");
                        return;
                    }
                    event::EventStatus::SUSPEND => {
                        log::debug!("Event wasn't finished, suspend.");
                        let mut guard = self.suspend_events.lock().unwrap();
                        guard.insert(event.get_task_id(), event);
                    }
                },
                None => {
                    panic!("event is None in queue")
                }
            }
        }
    }

    fn push_event(&self, mut event: event::Event<T>) -> Arc<EventHandler<T>> {
        if self.is_shutdown() {
            panic!("failed, thread was shutdowned");
        }

        let (tx, rx) = channel();

        event.set_tx(tx);

        let task_id = event.get_task_id();
        {
            let mut guard = self.suspend_events.lock().unwrap();
            guard.insert(task_id, event);

            log::debug!("event was added to suspended");
        }

        let event_handler = {
            let mut handlers_guard = self.handlers.lock().unwrap();
            let event_handler = EventHandler::<T>::new(rx);
            handlers_guard.insert(task_id, event_handler.clone());
            event_handler
        };

        self.resume_event(task_id);

        log::debug!("event {} was added to suspend_events", task_id);

        event_handler
    }

    fn resume_event(&self, task_id: waker::TWakerID) {
        let event = {
            let mut guard = self.suspend_events.lock().unwrap();
            guard.remove(&task_id).unwrap()
        };
        log::debug!("event {} was removed from suspended", event.get_task_id());

        let (q, cvar) = &*self.in_progress;
        let mut guard = q.lock().unwrap();
        guard.push_back(event);
        cvar.notify_one();
    }
}
