use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, RawWaker, RawWakerVTable};
use std::task::Waker;
use std::task::{Poll};
use std::pin::Pin;
use std::cell::RefCell;

use crate::reactor::Reactor;



struct FWrapper {
    id: i32,
    events: Arc<RefCell<Vec<i32>>>,
}

impl FWrapper {
    pub fn new(id: i32, events: Arc<RefCell<Vec<i32>>>,) -> Self {
        Self {
            id,
            events
        }
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |ptr: *const()| -> RawWaker {
            let c = unsafe { Arc::from_raw(ptr as *const FWrapper) };
            RawWaker::new(Arc::into_raw(c) as *const(), &VTABLE)
        },
        |ptr: *const()| {
            log::debug!("call wake");
            let c = unsafe { Arc::from_raw(ptr as *const FWrapper) };
            let mut es = c.events.borrow_mut();
            es.push(c.id);
        },
        |_| {

        },
        |_| {

        },
    );

pub struct Executor<F: Future> {
    reactor: Arc<Reactor>,
    table: HashMap<i32, Pin<Box<F>>>,
    events: Arc<RefCell<Vec<i32>>>,
}

impl<F: Future> Executor<F> {
    pub fn new() -> Self {
        Self {
            reactor: Arc::new(Reactor::new()),
            table: HashMap::new(),
            events: Arc::new(RefCell::new(vec![])),
        }
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        self.reactor.clone()
    }

    pub fn block_on(&mut self, future: F) -> F::Output
    {
        self.table.insert(1, Box::pin(future));

        let wrapper = Arc::new(FWrapper::new(1, self.events.clone()));

        let raw = RawWaker::new(
            Arc::into_raw(wrapper) as *const(), 
            &VTABLE
        );
        let waker = unsafe { Waker::from_raw(raw) };

        let mut ctx = Context::from_waker(&waker);

        let future = self.table.get_mut(&1).unwrap();
        if let Poll::Ready(output) = future.as_mut().poll(&mut ctx) {
            log::debug!("Executor: future is ready without select call, block_on is completed.");
            return output;
        }

        loop {
            log::debug!("Executor: next pool_once");
            self.reactor.poll_once();

            let es = self.events.borrow();
            if !es.is_empty() {
                log::debug!("Executor: there is an event.");

                for id in es.iter() {
                    let future = self.table.get_mut(id);

                    if let None = future {
                        log::warn!("Executor: empty future for id={}", *id);
                        continue;
                    }

                    match future.unwrap().as_mut().poll(&mut ctx) {
                        Poll::Ready(output) => {
                            log::debug!("Executor: future is ready, block_on is completed.");
                            return output;
                        }
                        Poll::Pending => {
                            log::debug!("Executor: future is pending, pool once.");
                        }
                    }
                }
            }
            


        }
    }
}
