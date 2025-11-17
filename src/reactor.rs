use std::collections::HashMap;
use std::mem;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

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

    pub fn remove_reader(&self, fd: i32) {
        let _ = self.fds.lock().unwrap().remove(&fd);
    }

    pub fn set_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    pub fn run_loop(&self) {
        loop {
            self.poll_once();
            if self.is_shutdown() {
                log::debug!("Compliting reactor thread.");
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    fn poll_once(&self) {
        log::trace!("Next pool_once.");
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
                let err = std::io::Error::last_os_error();

                if err.raw_os_error() == Some(libc::EBADF) {
                    log::error!("Syscall select finished with -1, found broken fd.");
                }

                if err.raw_os_error() == Some(libc::EINTR) {
                    log::error!("Syscall select finished with -1, unexpected signal.");
                }
            }
            0 => {
                log::trace!("Syscall select finished with timeout.");
            }
            count => {
                log::info!("Syscall select finished with {count} ready descriptors.");

                let guard = self.fds.lock().unwrap();
                guard
                    .iter()
                    .filter(|&(&fd, _)| unsafe { libc::FD_ISSET(fd, &readfds) })
                    .for_each(|(&fd, w)| {
                        log::debug!("descriptor {} is ready for read, call his waker.", fd);
                        w.clone().wake();
                    });

                // guard.retain(|&fd, w| {
                //     if unsafe { libc::FD_ISSET(fd, &readfds) } {
                //         log::debug!("descriptor {} is ready for read, call his waker.", fd);
                //         w.clone().wake();
                //         return true;
                //     } else {
                //         return false;
                //     }
                // });
            }
        }
    }
}
