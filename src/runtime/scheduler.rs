use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use crate::runtime::event::{self, PlanningPolicy};

use super::event::TEventID;
use super::event_handler::EventHandler;

pub(crate) struct Scheduler<T: Send + 'static> {
    inner: Arc<SchedulerImpl<T>>,
    events_handle: Cell<Option<JoinHandle<()>>>,
}

unsafe impl<T: Send + 'static> Sync for Scheduler<T> {}

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

    pub(crate) fn resume_event(&self, event_id: TEventID) {
        self.inner.resume_event(event_id);
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
    suspend_events: Mutex<HashMap<TEventID, event::Event<T>>>,
    handlers: Mutex<HashMap<TEventID, Arc<EventHandler<T>>>>,
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
                log::info!("thread was shutdowned, finish.");
                break;
            }

            log::trace!("loop_proccess try get next task.");

            let (lock, cvar) = &*self.in_progress;
            let guard = lock.lock().unwrap();
            let mut guard = cvar.wait_while(guard, |q| q.is_empty()).unwrap();

            let event = guard.pop_front();

            drop(guard);

            match event {
                Some(mut event) => {
                    log::trace!("try run the {event}.");

                    match event.proccess() {
                        event::EventStatus::READY(output) => {
                            log::info!("{event} is ready, data will sent to event.");
                            event.send_to_tx(output);
                            // return;
                        }
                        event::EventStatus::SUSPEND => {
                            log::debug!("{event} wasn't finished, suspend.");
                            let policy = event.get_rescheduler_policy().clone();
                            let event_id = event.get_event_id();

                            {
                                let mut suspend_events = self.suspend_events.lock().unwrap();
                                suspend_events.insert(event_id, event);
                            }

                            match policy {
                                PlanningPolicy::Internal => {
                                    self.resume_event(event_id);
                                }
                                PlanningPolicy::External => {}
                            }
                            // update reactor
                        }
                    }
                }
                None => {
                    panic!("event is None in queue")
                }
            }
        }
    }

    fn push_event(&self, mut event: event::Event<T>) -> Arc<EventHandler<T>> {
        if self.is_shutdown() {
            panic!("failed, thread was shutdowned.");
        }

        let (tx, rx) = channel();

        event.set_tx(tx);

        let event_id = event.get_event_id();
        {
            log::debug!("{event} is added to suspended.");

            let mut guard = self.suspend_events.lock().unwrap();
            guard.insert(event_id, event);
        }

        let event_handler = {
            let mut handlers_guard = self.handlers.lock().unwrap();
            let event_handler = EventHandler::<T>::new(event_id, rx);
            handlers_guard.insert(event_id, event_handler.clone());
            event_handler
        };

        self.resume_event(event_id);

        event_handler
    }

    fn resume_event(&self, task_id: TEventID) {
        let event = {
            let mut guard = self.suspend_events.lock().unwrap();
            guard.remove(&task_id)
        };

        if let None = event {
            log::debug!(
                "Event with task_id={task_id} not found in suspend_events. Probably the event is in in_progress."
            );
            return;
        }

        let event = event.expect("check early");

        log::debug!("{event} was removed from suspended and adding to in_progress.");

        let (q, cvar) = &*self.in_progress;
        let mut guard = q.lock().unwrap();
        guard.push_back(event);
        cvar.notify_one();
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::event::Event;
    use crate::runtime::event::PlanningPolicy;
    use crate::runtime::scheduler::Scheduler;
    use std::task::Waker;

    #[test]
    fn test_internal_event() {
        let scheduler = Scheduler::new();
        scheduler.activate();

        let event = Event::new(
            1,
            async move { 43 },
            Waker::noop().clone(),
            PlanningPolicy::Internal,
        );

        let handler = scheduler.push_event(event);
        let result = handler.wait_result();

        assert_eq!(result, 43);

        scheduler.deactivate();
    }
}
