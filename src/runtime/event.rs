use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::Sender;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

pub(crate) type TEventID = u64;

pub enum EventStatus<T> {
    READY(T),
    SUSPEND,
}

pub struct Event<T> {
    event_id: TEventID,
    future: Pin<Box<dyn Future<Output = T>>>,
    waker: Waker,
    tx: Option<Sender<T>>,
}

unsafe impl<T> Send for Event<T> {}

impl<T> Event<T> {
    pub fn new<F>(event_id: TEventID, future: F, waker: Waker) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        Self {
            event_id,
            future: Box::pin(future),
            waker,
            tx: None,
        }
    }

    pub fn get_event_id(&self) -> TEventID {
        self.event_id
    }

    pub fn set_tx(&mut self, tx: Sender<T>) {
        self.tx = Some(tx);
    }

    pub fn send_to_tx(&self, data: T) {
        match &self.tx {
            Some(tx) => {
                log::debug!("Send the data from {self} to handler.");
                if let Err(er) = tx.send(data) {
                    log::debug!("Failed send result from event: {}", er);
                    panic!("Failed send data");
                }
            }
            None => {
                panic!("tx wasn't seted");
            }
        }
    }

    pub fn proccess(&mut self) -> EventStatus<T> {
        let mut ctx = Context::from_waker(&self.waker);

        match self.future.as_mut().poll(&mut ctx) {
            Poll::Ready(output) => {
                log::debug!("{self} is ready, proccess is completed.");
                return EventStatus::READY(output);
            }
            Poll::Pending => {
                log::debug!("{self} is pending, re-schedule the event.");
                return EventStatus::SUSPEND;
            }
        }
    }
}

impl<T> Display for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Event(event_id={}, tx={})",
            self.event_id,
            self.tx.is_some()
        )
    }
}
