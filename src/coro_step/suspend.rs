use std::cell::Cell;
use std::fmt::Display;

pub struct CoroStepSuspend {
    ready: Cell<bool>,
}

impl CoroStepSuspend {
    pub fn execute() -> Self {
        Self {
            ready: Cell::new(false),
        }
    }
}

impl Drop for CoroStepSuspend {
    fn drop(&mut self) {
        // log::info!("Drop: remove cfd={} from reactor.", self.cfd);
        // self.reactor.remove_reader(self.cfd);
    }
}

impl Future for CoroStepSuspend {
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

impl Display for CoroStepSuspend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CoroStepSuspend(ready={})", self.ready.get())
    }
}
