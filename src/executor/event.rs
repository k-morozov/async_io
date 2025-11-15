use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

pub struct EventHandler {}

pub struct Event<F: Future + Send + 'static> {
    future: Pin<Box<F>>,
    waker: Waker,
    // ctx: Context<'static>,
    // handler: &'static EventHandler,
}

impl<F: Future + Send + 'static> Event<F>
where
    F: Future + Send + 'static,
{
    pub fn new(future: F, waker: Waker) -> Self {
        Self {
            future: Box::pin(future),
            waker,
        }
    }

    pub fn run(&mut self) {
        let mut ctx = Context::from_waker(&self.waker);

        match self.future.as_mut().poll(&mut ctx) {
            Poll::Ready(output) => {
                log::debug!("Event is ready, run is completed.");
                // return output;
            }
            Poll::Pending => {
                log::debug!("Future is pending, re-schedule the event.");
            }
        }
    }
}
