use std::sync::{Arc, mpsc};
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

pub(crate) type TWakerID = u64;
pub(crate) type TEvents = mpsc::Sender<TWakerID>;

pub struct WakerImpl {
    id: TWakerID,
    events: TEvents,
}

impl WakerImpl {
    pub fn new(id: TWakerID, events: TEvents) -> Self {
        Self { id, events }
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |ptr: *const ()| -> RawWaker {
        let c = unsafe { Arc::from_raw(ptr as *const WakerImpl) };
        RawWaker::new(Arc::into_raw(c) as *const (), &VTABLE)
    },
    |ptr: *const ()| {
        log::debug!("VTABLE: call wake");
        let c = unsafe { Arc::from_raw(ptr as *const WakerImpl) };

        let r = c.events.send(c.id);
        if let Err(e) = r {
            log::error!("VTABLE: Failed to send {} from waker: {e}", c.id);
        }
    },
    |_| {},
    |_| {},
);

pub fn make(id: TWakerID, events: TEvents) -> Waker {
    let waker = Arc::new(WakerImpl::new(id, events));

    let raw_waker = RawWaker::new(Arc::into_raw(waker) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}
