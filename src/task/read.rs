use std::cell::Cell;
use std::fmt::Display;
use std::sync::Arc;

use crate::reactor::Reactor;

pub struct Read {
    cfd: i32,
    nbytes: usize,
    reactor: Arc<Reactor>,

    registred: Cell<bool>,
}

impl Read {
    pub fn new(reactor: Arc<Reactor>, cfd: i32, nbytes: usize) -> Self {
        Self {
            cfd,
            nbytes,
            reactor,
            registred: Cell::new(false),
        }
    }
}

impl Drop for Read {
    fn drop(&mut self) {
        log::info!("Drop: remove cfd={} from reactor.", self.cfd);
        self.reactor.remove_reader(self.cfd);
    }
}

impl Future for Read {
    type Output = Vec<u8>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        log::debug!("{self} is called.");

        if !self.registred.get() {
            log::debug!("{self} was not registred, add cfd {} to reactor.", self.cfd);

            self.reactor.add_reader(self.cfd, cx.waker().clone());
            self.registred.set(true);

            log::debug!("{self} was registred and is being pending.");
            return std::task::Poll::Pending;
        }

        log::debug!("{self} was registred, prepare to CoroStepRead.");

        let mut buf = vec![0u8; self.nbytes];
        let bytes =
            unsafe { libc::read(self.cfd, buf.as_mut_ptr() as *mut libc::c_void, self.nbytes) };

        log::debug!("{self} read {} bytes.", bytes);

        if bytes == -1 {
            return std::task::Poll::Pending;
        }

        std::task::Poll::Ready(buf)
    }
}

impl Display for Read {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TaskRead(cfd={}, registred={})",
            self.cfd,
            self.registred.get()
        )
    }
}
