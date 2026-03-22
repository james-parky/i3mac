mod log;
pub mod logical;
pub mod physical;
mod registry;
mod tests;

use crate::display::log::Message::{
    AddLogical, AddPhysical, AddedWindow, AddingWindow, ChoseNewLogicalId, CouldNotFitWindow,
    FocusLogical, NoNewLogicalIds, RemovedEmptyLogical, RemovedWindow, SetActivePhysical, Split,
    SwitchToLogical,
};
use crate::display::logical::SomeWindows;
use crate::display::registry::Registry;
use crate::log::{Level, Log, Prefix};
use crate::{
    container::{Axis, Window},
    error::Error,
    error::Result,
    log::Logger,
};
use core_graphics::{Bounds, WindowId};
use std::collections::{HashMap, HashSet};
use std::io::Write;

pub struct Displays<S> {
    physical_displays: HashMap<physical::Id, physical::Display>,
    registry: Registry,
    logger: Logger,
    state: S,
}

pub struct Uninitialised;
pub struct Initialised {
    active_physical_display_id: physical::Id,
}

impl Displays<Uninitialised> {
    pub fn new() -> Self {
        Self {
            physical_displays: Default::default(),
            registry: Registry::new(),
            // TODO: get log level from a display::Config?
            logger: Logger::try_new("/dev/stdout", Level::Trace, Prefix::DISPLAY_MANAGER).unwrap(),
            state: Uninitialised,
        }
    }

    pub fn add_first_physical(
        self,
        pid: physical::Id,
        bounds: Bounds,
        cfg: physical::Config,
    ) -> Result<Displays<Initialised>> {
        let lid = self
            .registry
            .next_available_logical()
            .ok_or(Error::NoAvailableLogical)?;

        let pd = physical::Display::new(pid, lid, bounds, cfg);

        let physical_displays = HashMap::from_iter([(pid, pd)]);
        let mut registry = Registry::new();
        registry.register(lid, pid);

        let ret = Displays {
            physical_displays,
            registry,
            logger: self.logger,
            state: Initialised {
                active_physical_display_id: pid,
            },
        };

        Ok(ret)
    }
}

impl Displays<Initialised> {
    /// Returns the ID of the physical display that manages the provided window.
    pub fn display_of_window(&self, wid: WindowId) -> Option<physical::Id> {
        self.physical_displays
            .iter()
            .find(|(_, pd)| pd.window_ids().contains(&wid))
            .map(|(pid, _)| *pid)
    }

    /// Returns the ID of the physical display that manages the provided logical
    /// display.
    pub fn logical_id_owner(&self, id: logical::Id) -> Option<physical::Id> {
        self.registry.owner_of(id)
    }

    /// Returns a reference to the logical display corresponding to the provided
    /// logical display ID.
    // pub fn get_logical(&self, id: logical::Id) -> Option<&logical::Display> {
    //     let pid = self.logical_id_owner(id)?;
    //     self.physical_displays.get(&pid)?.logical(id)
    // }

    pub fn focus_display(&mut self, id: logical::Id) -> Option<WindowId> {
        if !self.registry.exists(id) {
            let pid = self.state.active_physical_display_id;
            self.physical_displays
                .get_mut(&pid)
                .unwrap()
                .create_logical_display(id);
            self.registry.register(id, pid);
        }

        // Safety: we just created it
        let pid = self.registry.owner_of(id).unwrap();
        self.state.active_physical_display_id = pid;
        self.physical_displays.get(&pid).unwrap().focused_window()
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.physical_displays
            .get_mut(&self.state.active_physical_display_id)
            .unwrap()
            .split(axis)?;

        Split(axis).log(&mut self.logger);
        Ok(())
    }

    pub fn set_active_physical_display(&mut self, id: physical::Id) {
        self.state.active_physical_display_id = id;
        SetActivePhysical(id).log(&mut self.logger);
    }

    pub fn physical_displays(&self) -> &HashMap<physical::Id, physical::Display> {
        &self.physical_displays
    }

    pub fn switch_logical_display(&mut self, pid: physical::Id, new_lid: logical::Id) {
        SwitchToLogical(pid, new_lid).log(&mut self.logger);

        let pd = self.physical_displays.get_mut(&pid).unwrap();
        let old_lid = pd.active_logical_id();
        pd.switch_to(new_lid);

        if pd.logical_is_empty(old_lid) {
            pd.remove_logical_display(old_lid);
            self.registry.deregister(old_lid);
            RemovedEmptyLogical(old_lid).log(&mut self.logger);
        }
    }

    pub fn add_physical(&mut self, pid: physical::Id, bounds: Bounds, cfg: physical::Config) {
        let lid = match self.registry.next_available_logical() {
            Some(x) => x,
            // TODO: error
            None => return,
        };

        let pd = physical::Display::new(pid, lid, bounds, cfg);

        self.physical_displays.insert(pid, pd);
        self.registry.register(lid, pid);

        self.state.active_physical_display_id = pid;
        AddPhysical(pid, lid).log(&mut self.logger);
    }

    pub fn get_occupied_logical(&self, lid: logical::Id) -> Option<&logical::Display<SomeWindows>> {
        let pid = self.registry.owner_of(lid)?;
        self.physical_displays.get(&pid)?.occupied_logical(lid)
    }

    pub(crate) fn active_logical_display_id(&self) -> logical::Id {
        self.physical_displays
            .get(&self.state.active_physical_display_id)
            .unwrap()
            .active_logical_id()
    }

    pub fn create_logical_display(
        &mut self,
        pid: physical::Id,
        lid: logical::Id,
    ) -> Result<logical::Id> {
        // TODO: is it fine to just require this being true from the caller?
        if self.registry.exists(lid) {
            return Err(Error::LogicalAlreadyExists(lid));
        }

        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .create_logical_display(lid);

        self.registry.register(lid, pid);

        AddLogical(pid, lid).log(&mut self.logger);
        Ok(lid)
    }

    pub fn physical_display_mut(&mut self, pid: physical::Id) -> Option<&mut physical::Display> {
        self.physical_displays.get_mut(&pid)
    }

    pub fn add_window(&mut self, window: Window) -> Result<AddWindowResult> {
        let initial_lid = self.active_logical_display_id();
        let mut lid = initial_lid;
        let pid = self.registry.owner_of(lid).unwrap();
        AddingWindow(window.id, pid).log(&mut self.logger);

        loop {
            match self
                .physical_displays
                .get_mut(&pid)
                .unwrap()
                .add_window_to_logical(window, lid)
            {
                Err(Error::CannotFitWindow) => {
                    CouldNotFitWindow(window.id, lid).log(&mut self.logger);
                    lid = self.registry.next_available_logical().unwrap();
                    self.physical_displays
                        .get_mut(&pid)
                        .unwrap()
                        .create_logical_display(lid);
                    self.registry.register(lid, pid);
                }
                Err(e) => {
                    return Err(e);
                }
                _ => break,
            }
        }

        AddedWindow(window.id, lid).log(&mut self.logger);
        if lid == initial_lid {
            Ok(AddWindowResult::Active(lid))
        } else {
            Ok(AddWindowResult::Overflow(lid))
        }
    }

    pub fn add_window_to_logical(&mut self, window: Window, lid: logical::Id) -> Result<()> {
        // let pid = *self.active_logical_display_ids.get(&lid).unwrap();
        //
        // // If the logical exists, add to it, otherwise first create it on pd
        // let pd = self.physical_displays.get_mut(&pid).unwrap();
        //
        // if !pd.has_logical_display(lid) {
        //     pd.create_logical_display(lid);
        // }
        //
        // pd.add_window_to_logical(window, lid)
        // If the target LD doesn't exist yet, create it on the PD that owns the
        // current active LD. Do NOT use active_physical_display_id directly —
        // it can be stale if the window being moved was on a different PD.
        if !self.registry.exists(lid) {
            let pid = self
                .physical_displays
                .iter()
                .find(|(_, pd)| pd.has_logical_display(self.active_logical_display_id()))
                .map(|(pid, _)| *pid)
                .unwrap(); // active LD always has an owner
            self.create_logical_display(pid, lid)?;
        }

        let pid = self.registry.owner_of(lid).unwrap();
        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .add_window_to_logical(window, lid)?;

        AddedWindow(window.id, lid).log(&mut self.logger);
        Ok(())
    }

    pub fn remove_window(&mut self, pid: physical::Id, wid: WindowId) -> Result<()> {
        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .remove_window(wid)?;

        RemovedWindow(wid, pid).log(&mut self.logger);
        Ok(())
    }

    pub fn logical_ids(&self, pid: physical::Id) -> HashSet<logical::Id> {
        // TODO: return Iterator<Item = logical::Id> here too?
        self.registry.logicals(pid).collect()
    }

    pub fn active_physical_display_mut(&mut self) -> &mut physical::Display {
        self.physical_displays
            .get_mut(&self.state.active_physical_display_id)
            .unwrap()
    }
}

pub enum AddWindowResult {
    Active(logical::Id),
    Overflow(logical::Id),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::log::{Level, Prefix};

    fn pid(id: usize) -> physical::Id {
        physical::Id(id)
    }

    fn lid(id: usize) -> logical::Id {
        logical::Id(id)
    }

    fn bounds() -> Bounds {
        Bounds {
            height: 100.0,
            width: 10.0,
            x: 0.0,
            y: 0.0,
        }
    }

    impl Default for Displays<Uninitialised> {
        fn default() -> Self {
            Self {
                physical_displays: Default::default(),
                registry: Registry::new(),
                logger: Logger::try_new("/dev/null", Level::Error, Prefix::DISPLAY_MANAGER)
                    .unwrap(),
                state: Uninitialised,
            }
        }
    }

    #[test]
    fn add_physical_creates_a_logical() {
        let d = Displays::default();

        let pid = pid(0);

        let mut d = d
            .add_first_physical(pid, bounds(), Default::default())
            .unwrap();

        // There are no logical displays, so the first should be 0.
        d.add_physical(pid, bounds(), Default::default());

        let lids = d.logical_ids(pid);

        assert_eq!(lids.len(), 1);
        assert_eq!(d.active_logical_display_id(), *lids.iter().next().unwrap());
        assert_eq!(d.active_logical_display_id(), lid(0));
    }
}
