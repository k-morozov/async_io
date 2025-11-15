use std::cell::Cell;
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

    // pub fn register(&self) {
    //     if !self.registred.get() {
    //         log::debug!(
    //             "Task was not registred, add cfd {} to reactor",
    //             self.cfd
    //         );

    //         self.reactor.add_reader(self.cfd, cx.waker().clone());
    //         self.registred.set(true);

    //         log::debug!("Task was registred and is being pending");
    //     }
    // }

    pub fn reset(&self) {
        let mut readfds = self.readfds.borrow_mut();

        *readfds = unsafe { mem::zeroed() };
        unsafe {
            libc::FD_ZERO(readfds.deref_mut());
            libc::FD_SET(self.cfd, readfds.deref_mut());
        }
    }
}

impl Future for Task {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        log::debug!("Task is called for cfd={}", self.cfd);

        if !self.registred.get() {
            log::debug!("Task was not registred, add cfd {} to reactor", self.cfd);

            self.reactor.add_reader(self.cfd, cx.waker().clone());
            self.registred.set(true);

            log::debug!("Task was registred and is being pending");
            return std::task::Poll::Pending;
        }

        log::debug!("Task was registred, prepare to read");

        const MSG_LEN: usize = 4;

        let mut buf = [0u8; MSG_LEN];
        let bytes = unsafe { libc::read(self.cfd, buf.as_mut_ptr() as *mut libc::c_void, MSG_LEN) };

        log::debug!("Task read {} bytes", bytes);

        if bytes == -1 {
            return std::task::Poll::Pending;
        }

        let result = String::from_utf8_lossy(&buf[..bytes as usize]).to_string();
        log::debug!("Task has result buf: {:?}", result);

        log::debug!("Task was finished");
        std::task::Poll::Ready(())
    }
}
