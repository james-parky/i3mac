use std::os::raw::c_int;

pub struct Pipe {
    pub read_fd: c_int,
    pub write_fd: c_int,
}

impl Pipe {
    pub fn new() -> Self {
        let mut fds = [0i32; 2];
        unsafe {
            libc::pipe(fds.as_mut_ptr());
            libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK);
            libc::fcntl(fds[1], libc::F_SETFL, libc::O_NONBLOCK);
        }
        Self {
            read_fd: fds[0],
            write_fd: fds[1],
        }
    }

    pub fn drain(&self) {
        let mut buf = [0u8; 64];
        unsafe { libc::read(self.read_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.read_fd);
            libc::close(self.write_fd);
        }
    }
}
