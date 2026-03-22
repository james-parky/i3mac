mod error;
mod keyboard;
mod kqueue;
mod mux;
mod observer;
mod pipe;

use crate::poll::error::Error;
use core_graphics::Error::NoneAvailable;
use core_graphics::KeyCommand;
use libc::{intptr_t, kevent};
pub use mux::Mux;
pub use observer::*;
pub use pipe::Pipe;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::{
    os::fd::RawFd,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

pub type Result<T> = std::result::Result<T, Error>;
use crate::poll::kqueue::KQueue;
pub use keyboard::*;

pub enum Event {
    Keyboard(Vec<KeyCommand>),
    Workspace(Vec<WorkspaceEvent>),
    CtlMsg { rx: Vec<u8>, reply: UnixStream },
    Timer,
}

pub type Ident = usize;

pub trait Source {
    type Event;

    fn poll(&self) -> Option<Self::Event>;
    fn as_kevent(&self) -> libc::kevent;
    fn ident(&self) -> Ident;
}

pub trait PollSources {
    type Event;

    fn register(&self, kq: &KQueue);
    fn poll_ident(&self, ident: Ident) -> Option<Self::Event>;
}

pub struct Base<Event>(std::marker::PhantomData<Event>);

impl<Event> PollSources for Base<Event> {
    type Event = Event;

    fn register(&self, _: &KQueue) {}
    fn poll_ident(&self, _: Ident) -> Option<Self::Event> {
        None
    }
}

impl<Head, Tail> PollSources for (Head, Tail)
where
    Head: Source,
    Tail: PollSources<Event = Head::Event>,
{
    type Event = Head::Event;

    fn register(&self, kq: &KQueue) {
        kq.add(&self.0.as_kevent());
        self.1.register(kq);
    }

    fn poll_ident(&self, ident: Ident) -> Option<Self::Event> {
        if ident == self.0.ident() {
            self.0.poll()
        } else {
            self.1.poll_ident(ident)
        }
    }
}

pub struct Timer {
    pub id: usize,
    pub interval: Duration,
}

impl Source for Timer {
    type Event = Event;

    fn poll(&self) -> Option<Self::Event> {
        Some(Event::Timer)
    }

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

impl Source for UnixListener {
    type Event = Event;

    fn poll(&self) -> Option<Self::Event> {
        let mut buf = Vec::with_capacity(20);
        let (mut stream, _) = self.accept().unwrap();
        let _ = stream.read_to_end(&mut buf).unwrap();
        Some(Event::CtlMsg {
            rx: buf,
            reply: stream,
        })
    }

    fn as_kevent(&self) -> kevent {
        libc::kevent {
            ident: self.as_raw_fd() as usize,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        }
    }

    fn ident(&self) -> Ident {
        self.as_raw_fd() as usize
    }
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

impl Source for ChannelSource<WorkspaceEvent> {
    type Event = Event;

    fn poll(&self) -> Option<Self::Event> {
        let events = ChannelSource::drain(self);
        if events.is_empty() {
            None
        } else {
            Some(Event::Workspace(events))
        }
    }

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

impl Source for ChannelSource<KeyCommand> {
    type Event = Event;

    fn poll(&self) -> Option<Event> {
        let commands = ChannelSource::drain(self);
        if commands.is_empty() {
            None
        } else {
            Some(Event::Keyboard(commands))
        }
    }

    fn as_kevent(&self) -> libc::kevent {
        libc::kevent {
            ident: self.ident(),
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
