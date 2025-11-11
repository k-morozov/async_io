use std::cell::Cell;
use std::mem;
use std::ops::DerefMut;
use std::sync::Arc;

use crate::reactor::Reactor;

pub struct Handler {
    cfd: i32,
    reactor: Arc<Reactor>,
    readfds: std::cell::RefCell<libc::fd_set>,

    registred: Cell<bool>,
}

impl Handler {
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

impl Future for Handler {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        log::debug!("Handler::poll is called");

        if !self.registred.get() {
            log::debug!(
                "Handler::poll was not registred, add cfd {} to reactor",
                self.cfd
            );

            self.reactor.add_reader(self.cfd, cx.waker().clone());
            self.registred.set(true);

            log::debug!("Handler::poll is pending");
            return std::task::Poll::Pending;
        }

        log::debug!("Handler::poll was registred, prepare to read");

        const MSG_LEN: usize = 4;

        let mut buf = [0u8; MSG_LEN];
        let bytes = unsafe { libc::read(self.cfd, buf.as_mut_ptr() as *mut libc::c_void, MSG_LEN) };

        log::debug!("Handler::poll read {} bytes", bytes);

        if bytes == -1 {
            return std::task::Poll::Pending;
        }

        let result = String::from_utf8_lossy(&buf[..bytes as usize]).to_string();
        log::debug!("Handler::poll has result buf: {:?}", result);

        log::debug!("Handler::poll was finished");
        std::task::Poll::Ready(())
    }
}
