use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use super::waker;

pub enum EventStatus<T> {
    READY(T),
    SUSPEND,
}

pub struct Event<F: Future + Send + 'static> {
    task_id: waker::TWakerID,
    future: Pin<Box<F>>,
    waker: Waker,
    tx: Option<Sender<F::Output>>,
}

impl<F: Future + Send + 'static> Event<F>
where
    F: Future + Send + 'static,
{
    pub fn new(task_id: waker::TWakerID, future: F, waker: Waker) -> Self {
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

    pub fn set_tx(&mut self, tx: Sender<F::Output>) {
        self.tx = Some(tx);
    }

    pub fn send_to_tx(&self, data: F::Output) {
        match &self.tx {
            Some(tx) => {
                log::debug!("send the data");
                tx.send(data);
            },
            None => todo!(),
        }
    }

    pub fn run(&mut self) -> EventStatus<F::Output> {
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
