use std::fmt::Display;
use std::sync::Arc;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

use super::event::TEventID;

pub struct WakerImpl {
    event_id: TEventID,
    resume: Box<dyn Fn(TEventID)>,
}

impl WakerImpl {
    pub fn new(event_id: TEventID, resume: Box<dyn Fn(TEventID)>) -> Self {
        Self { event_id, resume }
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |ptr: *const ()| -> RawWaker {
        let c = unsafe { Arc::from_raw(ptr as *const WakerImpl) };
        let waker = c.clone();
        std::mem::forget(c);

        RawWaker::new(Arc::into_raw(waker) as *const (), &VTABLE)
    },
    |ptr: *const ()| {
        let c = unsafe { Arc::from_raw(ptr as *const WakerImpl) };

        log::debug!("{c} resume event.");
        (c.resume)(c.event_id);
    },
    |_| {},
    |_| {},
);

pub fn make(event_id: TEventID, resume: Box<dyn Fn(TEventID)>) -> Waker {
    let waker = Arc::new(WakerImpl::new(event_id, resume));

    let raw_waker = RawWaker::new(Arc::into_raw(waker) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

impl Display for WakerImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WakerImpl(event_id={}).", self.event_id)
    }
}
