use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::Sender;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use super::waker;

pub enum EventStatus<T> {
    READY(T),
    SUSPEND,
}

pub struct Event<T> {
    task_id: waker::TWakerID,
    future: Pin<Box<dyn Future<Output = T>>>,
    waker: Waker,
    tx: Option<Sender<T>>,
}

unsafe impl<T> Send for Event<T> {}

impl<T> Event<T> {
    pub fn new<F>(task_id: waker::TWakerID, future: F, waker: Waker) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        Self {
            task_id,
            future: Box::pin(future),
            waker,
            tx: None,
        }
    }

    pub fn get_task_id(&self) -> waker::TWakerID {
        self.task_id
    }

    pub fn set_tx(&mut self, tx: Sender<T>) {
        self.tx = Some(tx);
    }

    pub fn send_to_tx(&self, data: T) {
        match &self.tx {
            Some(tx) => {
                log::debug!("send the data");
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

    pub fn run(&mut self) -> EventStatus<T> {
        let mut ctx = Context::from_waker(&self.waker);

        match self.future.as_mut().poll(&mut ctx) {
            Poll::Ready(output) => {
                log::debug!("Event is ready, run is completed.");
                return EventStatus::READY(output);
            }
            Poll::Pending => {
                log::debug!("Event is pending, re-schedule the event.");
                return EventStatus::SUSPEND;
            }
        }
    }
}
