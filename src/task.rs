use std::cell::Cell;
use std::fmt::Display;
use std::mem;
use std::ops::DerefMut;
use std::sync::Arc;

use crate::reactor::Reactor;

pub struct Task {
    cfd: i32,
    reactor: Arc<Reactor>,
    readfds: std::cell::RefCell<libc::fd_set>,

    registred: Cell<bool>,
}

impl Task {
    pub fn new(reactor: Arc<Reactor>, cfd: i32) -> Self {
        Self {
            cfd,
            reactor,
            readfds: std::cell::RefCell::new(unsafe { mem::zeroed() }),
            registred: Cell::new(false),
        }
    }

    pub fn reset(&self) {
        let mut readfds = self.readfds.borrow_mut();

        *readfds = unsafe { mem::zeroed() };
        unsafe {
            libc::FD_ZERO(readfds.deref_mut());
            libc::FD_SET(self.cfd, readfds.deref_mut());
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        log::info!("Drop: remove cfd={} from reactor", self.cfd);
        self.reactor.remove_reader(self.cfd);
    }
}

impl Future for Task {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        log::debug!("{self} is called");

        if !self.registred.get() {
            log::debug!("{self} was not registred, add cfd {} to reactor", self.cfd);

            self.reactor.add_reader(self.cfd, cx.waker().clone());
            self.registred.set(true);

            log::debug!("{self} was registred and is being pending");
            return std::task::Poll::Pending;
        }

        log::debug!("{self} was registred, prepare to read");

        const MSG_LEN: usize = 4;

        let mut buf = [0u8; MSG_LEN];
        let bytes = unsafe { libc::read(self.cfd, buf.as_mut_ptr() as *mut libc::c_void, MSG_LEN) };

        log::debug!("{self} read {} bytes", bytes);

        if bytes == -1 {
            return std::task::Poll::Pending;
        }

        let result = String::from_utf8_lossy(&buf[..bytes as usize]).to_string();
        log::debug!("{self} was finished, buf: {:?}", result);

        // @todo remove from reactor
        std::task::Poll::Ready(())
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task(cfd={}, registred={})",
            self.cfd,
            self.registred.get()
        )
    }
}
