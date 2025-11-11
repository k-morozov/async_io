use std::future::Future;
use std::sync::Arc;
use std::task::Context;
use std::task::Waker;

use crate::reactor::Reactor;

pub struct Executor(Arc<Reactor>);

impl Executor {
    pub fn new() -> Self {
        Self(Arc::new(Reactor::new()))
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.0.clone()
    }

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        let mut future = std::pin::pin!(future);

        let waker = Waker::noop().clone();
        let mut ctx = Context::from_waker(&waker);

        loop {
            match future.as_mut().poll(&mut ctx) {
                std::task::Poll::Ready(output) => {
                    log::debug!("Executor: future is ready, block_on is completed.");
                    return output;
                }
                std::task::Poll::Pending => {
                    log::debug!("Executor: future is pending, pool once.");
                    self.0.poll_once();
                }
            }
        }
    }
}
