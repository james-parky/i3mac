use crate::display::{logical, physical};
use std::collections::HashMap;

pub(super) struct Registry {
    /// A map of logical ID -> physical ID. This serves as a record of which
    /// logical displays exists AND which physical displays they exist on
    /// respectively.
    // TODO: be some const sized array of options; either exists(pid) or None?
    map: HashMap<logical::Id, physical::Id>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn register(&mut self, lid: logical::Id, pid: physical::Id) {
        self.map.insert(lid, pid);
    }

    pub fn deregister(&mut self, lid: logical::Id) {
        self.map.remove(&lid);
    }

    pub fn owner_of(&self, lid: logical::Id) -> Option<physical::Id> {
        self.map.get(&lid).copied()
    }

    pub fn exists(&self, lid: logical::Id) -> bool {
        self.map.contains_key(&lid)
    }

    pub fn logicals(&self, target: physical::Id) -> impl Iterator<Item = logical::Id> + '_ {
        self.map
            .iter()
            .filter_map(move |(lid, pid)| if target == *pid { Some(*lid) } else { None })
    }

    pub fn next_available_logical(&self) -> Option<logical::Id> {
        (0..9)
            .map(logical::Id)
            .find(|lid| !self.map.contains_key(lid))
    }
}
