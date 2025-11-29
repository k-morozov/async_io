use std::cell::Cell;
use std::fmt::Display;

pub struct Suspend {
    ready: Cell<bool>,
}

impl Suspend {
    pub fn new() -> Self {
        Self {
            ready: Cell::new(false),
        }
    }
}

impl Future for Suspend {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        log::debug!("{self} is called.");

        if self.ready.get() {
            return std::task::Poll::Ready(());
        }

        self.ready.set(true);
        std::task::Poll::Pending
    }
}

impl Display for Suspend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskSuspend(ready={})", self.ready.get())
    }
}
