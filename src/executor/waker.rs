use std::fmt::Display;
use std::sync::Arc;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

pub(crate) type TWakerID = u64;

pub struct WakerImpl {
    task_id: TWakerID,
    resume: Box<dyn Fn(TWakerID)>,
}

impl WakerImpl {
    pub fn new(task_id: TWakerID, resume: Box<dyn Fn(TWakerID)>) -> Self {
        Self { task_id, resume }
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

        log::debug!("{c} resume event");
        (c.resume)(c.task_id);
    },
    |_| {},
    |_| {},
);

pub fn make(task_id: TWakerID, resume: Box<dyn Fn(TWakerID)>) -> Waker {
    let waker = Arc::new(WakerImpl::new(task_id, resume));

    let raw_waker = RawWaker::new(Arc::into_raw(waker) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

impl Display for WakerImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WakerImpl(task_id={})", self.task_id)
    }
}
