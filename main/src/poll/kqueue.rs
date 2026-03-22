use crate::poll::{Error, Ident, Result};
use libc::c_int;

pub(super) struct KQueue {
    fd: c_int,
}

impl KQueue {
    pub fn new() -> Result<Self> {
        unsafe {
            match libc::kqueue() {
                x if x < 0 => Err(Error::FailedToCreateKQueue),
                fd => Ok(Self { fd }),
            }
        }
    }

    pub fn add(&self, event: &libc::kevent) {
        unsafe { libc::kevent(self.fd, event, 1, std::ptr::null_mut(), 0, std::ptr::null()) };
    }

    pub fn poll(&self) -> Vec<Ident> {
        const MAX: usize = 32;
        const EMPTY: libc::kevent = libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };

        let mut buf = [EMPTY; MAX];
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let n = unsafe {
            libc::kevent(
                self.fd,
                std::ptr::null(),
                0,
                buf.as_mut_ptr(),
                MAX as c_int,
                &timeout,
            )
        };

        if n <= 0 {
            return vec![];
        }

        buf[..n as usize]
            .iter()
            .filter_map(|e| match e.filter {
                libc::EVFILT_READ | libc::EVFILT_TIMER => Some(e.ident),
                _ => None,
            })
            .collect()
    }
}

impl Drop for KQueue {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}
