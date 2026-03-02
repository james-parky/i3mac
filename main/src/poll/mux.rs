use crate::{
    poll::Event,
    poll::error::Error,
    poll::{AsKEvent, Result},
};
use std::os::raw::c_int;

pub struct Mux {
    kq: c_int,
}

impl Mux {
    pub fn new() -> Result<Self> {
        unsafe {
            match libc::kqueue() {
                x if x < 0 => Err(Error::FailedToCreateMux),
                x => Ok(Self { kq: x }),
            }
        }
    }

    pub fn add<T: AsKEvent>(&self, event: &T) {
        use std::ptr::{null, null_mut};
        unsafe { libc::kevent(self.kq, &event.as_kevent(), 1, null_mut(), 0, null()) };
    }

    /// Return any currently pending events without blocking.
    pub fn poll(&self) -> Vec<Event> {
        self.run(Some(libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }))
    }

    fn run(&self, timeout: Option<libc::timespec>) -> Vec<Event> {
        const MAX: usize = 32;

        const EMPTY_KEVENT: libc::kevent = libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };

        let mut buf = [EMPTY_KEVENT; MAX];

        let timeout_ptr = match &timeout {
            Some(t) => t,
            None => std::ptr::null(),
        };

        let n = unsafe {
            libc::kevent(
                self.kq,
                std::ptr::null(),
                0,
                buf.as_mut_ptr(),
                MAX as c_int,
                timeout_ptr,
            )
        };

        if n <= 0 {
            return vec![];
        }

        buf[..n as usize]
            .iter()
            .filter_map(|e| match e.filter {
                libc::EVFILT_READ => Some(Event::Readable(e.ident)),
                libc::EVFILT_TIMER => Some(Event::Timer(e.ident)),
                _ => None,
            })
            .collect()
    }
}

impl Drop for Mux {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.kq);
        }
    }
}
