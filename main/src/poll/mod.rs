mod error;
mod keyboard;
mod mux;
mod observer;
mod pipe;

use crate::poll::error::Error;
use libc::{intptr_t, kevent};
pub use mux::Mux;
pub use observer::*;
pub use pipe::Pipe;
use std::{
    os::fd::RawFd,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};
pub type Result<T> = std::result::Result<T, Error>;
pub use keyboard::*;

pub type Ident = usize;

pub trait AsKEvent {
    fn as_kevent(&self) -> libc::kevent;
    fn ident(&self) -> Ident;
}

pub struct Timer {
    pub id: usize,
    pub interval: Duration,
}

impl AsKEvent for Timer {
    fn as_kevent(&self) -> kevent {
        libc::kevent {
            ident: self.id,
            filter: libc::EVFILT_TIMER,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: libc::NOTE_SECONDS,
            data: self.interval.as_secs() as intptr_t,
            udata: std::ptr::null_mut(),
        }
    }

    fn ident(&self) -> Ident {
        self.id
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Readable(Ident),
    Timer(Ident),
}

pub struct ChannelSource<T> {
    rx: Receiver<T>,
    pipe: Pipe,
}

#[derive(Clone)]
pub struct ChannelSender<T> {
    tx: Sender<T>,
    write_fd: RawFd,
}

impl<T> ChannelSource<T> {
    pub fn new() -> (Self, ChannelSender<T>) {
        let (tx, rx) = channel();
        let pipe = Pipe::new();
        let write_fd = pipe.write_fd;
        (Self { rx, pipe }, ChannelSender { tx, write_fd })
    }

    pub fn drain(&self) -> Vec<T> {
        self.pipe.drain();
        std::iter::from_fn(|| self.rx.try_recv().ok()).collect()
    }
}

impl<T> ChannelSender<T> {
    pub fn send(&self, value: T) {
        let _ = self.tx.send(value);
        let byte = [1u8];
        let ret = unsafe { libc::write(self.write_fd, byte.as_ptr().cast(), 1) };
        println!("sent to pipe fd {} ret: {}", self.write_fd, ret);
    }
}

impl<T> AsKEvent for ChannelSource<T> {
    fn as_kevent(&self) -> kevent {
        libc::kevent {
            ident: self.pipe.read_fd as usize,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        }
    }

    fn ident(&self) -> Ident {
        self.pipe.read_fd as Ident
    }
}
