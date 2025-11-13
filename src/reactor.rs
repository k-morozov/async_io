use std::collections::HashMap;
use std::mem;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Reactor {
    fds: Mutex<HashMap<i32, std::task::Waker>>,
    shutdown: AtomicBool,
}

unsafe impl Sync for Reactor {}

impl Reactor {
    pub fn new() -> Self {
        Self {
            fds: Mutex::new(HashMap::new()),
            shutdown: AtomicBool::new(false),
        }
    }

    pub fn add_reader(&self, fd: i32, waker: std::task::Waker) {
        self.fds.lock().unwrap().insert(fd, waker.clone());
    }

    pub fn set_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    pub fn poll_once(&self) {
        log::debug!("Next pool_once");
        if self.is_shutdown() {
            log::debug!("Reactor was shutdowned.");
            return;
        }

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
                log::error!("Syscall select finished with -1");
            }
            0 => {
                log::warn!("Syscall select finished with timeout");
            }
            count => {
                log::warn!("Syscall select finished with result {count}");
                for (fd, waker) in self.fds.lock().unwrap().iter() {
                    if unsafe { libc::FD_ISSET(*fd, &readfds) } {
                        log::debug!("wake {}", *fd);
                        waker.clone().wake();
                        // @todo
                        // self.fds.remove(fd);
                    }
                }
            }
        }
    }
}
