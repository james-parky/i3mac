use crate::poll::{Base, PollSources, Result, Source, kqueue::KQueue};
use std::marker::PhantomData;

pub struct Mux<Sources> {
    kq: KQueue,
    sources: Sources,
}

impl Mux<()> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            kq: KQueue::new()?,
            sources: (),
        })
    }

    pub fn with_first<New: Source>(self, source: New) -> Mux<(New, Base<New::Event>)> {
        self.kq.add(&source.as_kevent());
        Mux {
            kq: self.kq,
            sources: (source, Base(PhantomData)),
        }
    }
}

impl<Sources> Mux<Sources> {
    pub fn with<Event, New>(self, source: New) -> Mux<(New, Sources)>
    where
        New: Source<Event = Event>,
        Sources: PollSources<Event = Event>,
    {
        self.kq.add(&source.as_kevent());
        Mux {
            kq: self.kq,
            sources: (source, self.sources),
        }
    }
}

impl<Sources> Mux<Sources>
where
    Sources: PollSources,
{
    pub fn poll(&self) -> Vec<Sources::Event> {
        self.kq
            .poll()
            .into_iter()
            .filter_map(|ident| self.sources.poll_ident(ident))
            .collect()
    }
}
