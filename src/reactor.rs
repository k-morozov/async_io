use std::collections::HashMap;
use std::mem;
use std::sync::Mutex;

pub struct Reactor {
    fds: Mutex<HashMap<i32, std::task::Waker>>,
    // h: Cell<Option<JoinHandle<()>>>,
}

unsafe impl Sync for Reactor {}

impl Reactor {
    pub fn new() -> Self {
        Self {
            fds: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_reader(&self, fd: i32, waker: std::task::Waker) {
        self.fds.lock().unwrap().insert(fd, waker.clone());
    }

    pub fn poll_once(&self) {
        let mut readfds: libc::fd_set = unsafe { mem::zeroed() };
        let mut writefds: libc::fd_set = unsafe { mem::zeroed() };

        let mut nax_fd = 0;

        unsafe {
            libc::FD_ZERO(&mut readfds);

            for &fd in self.fds.lock().unwrap().keys() {
                libc::FD_SET(fd, &mut readfds);
                if fd > nax_fd {
                    nax_fd = fd;
                }
            }
        }

        let mut tm = libc::timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        match unsafe {
            libc::select(
                nax_fd + 1,
                &mut readfds,
                &mut writefds,
                std::ptr::null_mut(),
                &mut tm,
            )
        } {
            -1 => {
                log::error!("Reactor: select with -1");
            }
            0 => {
                log::warn!("Reactor: select with timeout");
            }
            count => {
                log::warn!("Reactor: select with result {count}");
                for (fd, waker) in self.fds.lock().unwrap().iter() {
                    if unsafe { libc::FD_ISSET(*fd, &readfds) } {
                        log::debug!("Reactor: wake {}", *fd);
                        waker.clone().wake();
                        // self.fds.remove(fd);
                    }
                }
            }
        }
    }
}
