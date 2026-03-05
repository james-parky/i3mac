pub mod logical;
pub mod physical;
mod tests;

use crate::container;
use crate::container::Axis;
use crate::error::Error;
use crate::error::Result;
use container::Window;
use core_graphics::{Bounds, WindowId};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;

#[derive(Default)]
pub struct Displays {
    physical_displays: HashMap<physical::Id, physical::Display>,
    active_logical_display_ids: HashMap<logical::Id, physical::Id>,
    active_physical_display_id: Option<physical::Id>,
}

impl Displays {
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
        self.active_logical_display_ids.get(&id).copied()
    }

    /// Returns a reference to the logical display corresponding to the provided
    /// logical display ID.
    pub fn get_logical(&self, id: logical::Id) -> Option<&logical::Display> {
        let pid = self.logical_id_owner(id)?;
        self.physical_displays.get(&pid)?.logical(id)
    }

    pub fn focus_display(&mut self, id: logical::Id) -> Option<WindowId> {
        let pid = *self.active_logical_display_ids.get(&id).unwrap();
        self.active_physical_display_id = Some(pid);
        self.physical_displays.get(&pid).unwrap().focused_window()
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.physical_displays
            .get_mut(&self.active_physical_display_id.unwrap())
            .unwrap()
            .split(axis)
    }

    pub fn set_active_physical_display(&mut self, id: physical::Id) {
        self.active_physical_display_id = Some(id);
    }

    pub fn physical_displays(&self) -> &HashMap<physical::Id, physical::Display> {
        &self.physical_displays
    }

    pub fn switch_logical_display(&mut self, pid: physical::Id, new_lid: logical::Id) {
        let pd = self.physical_displays.get(&pid).unwrap();

        let old_lid = pd.active_logical_id();

        let old_empty = pd
            .logical(old_lid)
            .map(|ld| ld.window_ids().is_empty())
            .unwrap_or(false);

        let pd = self.physical_displays.get_mut(&pid).unwrap();
        pd.switch_to(new_lid);

        if old_empty {
            pd.remove_logical_display(old_lid);
            self.active_logical_display_ids.remove(&old_lid);
        }
    }

    pub fn next_logical_display_id(&mut self, pid: physical::Id) -> Option<logical::Id> {
        for id in 0..=9 {
            let lid = logical::Id(id);
            if let std::collections::hash_map::Entry::Vacant(e) =
                self.active_logical_display_ids.entry(lid)
            {
                e.insert(pid);
                return Some(lid);
            }
        }

        None
    }

    pub fn add_physical(&mut self, pid: physical::Id, bounds: Bounds, cfg: physical::Config) {
        let next_logical_id = self
            .next_logical_display_id(pid)
            .expect("already have 10 logical displays");

        let pd = physical::Display::new(next_logical_id, bounds, cfg);

        self.physical_displays.insert(pid, pd);
        self.active_logical_display_ids.insert(next_logical_id, pid);

        self.active_physical_display_id = Some(pid);
    }

    pub(crate) fn active_logical_display_id(&self) -> logical::Id {
        self.physical_displays
            .get(&self.active_physical_display_id.unwrap())
            .unwrap()
            .active_logical_id()
    }

    pub fn create_logical_display(&mut self, pid: physical::Id, lid: logical::Id) -> logical::Id {
        assert!(!self.active_logical_display_ids.contains_key(&lid));

        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .create_logical_display(lid);

        self.active_logical_display_ids.insert(lid, pid);

        lid
    }

    pub fn physical_display_mut(&mut self, pid: physical::Id) -> Option<&mut physical::Display> {
        self.physical_displays.get_mut(&pid)
    }

    pub fn add_window(&mut self, window: Window) -> Result<AddWindowResult> {
        let initial_lid = self.active_logical_display_id();
        let mut lid = initial_lid;
        let pid = *self.active_logical_display_ids.get(&lid).unwrap();

        loop {
            match self
                .physical_displays
                .get_mut(&pid)
                .unwrap()
                .add_window_to_logical(window, lid)
            {
                Err(Error::CannotFitWindow) => {
                    lid = self.next_logical_display_id(pid).unwrap();
                    self.physical_displays
                        .get_mut(&pid)
                        .unwrap()
                        .create_logical_display(lid);
                    self.active_logical_display_ids.insert(lid, pid);
                }
                Err(e) => {
                    return Err(e);
                }
                _ => break,
            }
        }

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
        if !self.active_logical_display_ids.contains_key(&lid) {
            let pid = self
                .physical_displays
                .iter()
                .find(|(_, pd)| pd.has_logical_display(self.active_logical_display_id()))
                .map(|(pid, _)| *pid)
                .unwrap(); // active LD always has an owner
            self.create_logical_display(pid, lid);
        }

        let pid = *self.active_logical_display_ids.get(&lid).unwrap();
        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .add_window_to_logical(window, lid)
    }

    pub fn remove_window(&mut self, pid: physical::Id, wid: WindowId) -> Result<Option<WindowId>> {
        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .remove_window(wid)
    }

    pub fn logical_ids(&self, pid: physical::Id) -> HashSet<logical::Id> {
        self.active_logical_display_ids
            .iter()
            .filter_map(|(l, p)| if *p == pid { Some(*l) } else { None })
            .collect()
    }

    pub fn active_physical_display_mut(&mut self) -> &mut physical::Display {
        self.physical_displays
            .get_mut(&self.active_physical_display_id.unwrap())
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

    #[test]
    fn add_physical_creates_a_logical() {
        let mut d = Displays::default();

        let pid = pid(0);

        // There are no logical displays, so the first should be 0.
        d.add_physical(pid, bounds(), Default::default());

        let lids = d.logical_ids(pid);

        assert_eq!(lids.len(), 1);
        assert_eq!(d.active_logical_display_id(), *lids.iter().next().unwrap());
        assert_eq!(d.active_logical_display_id(), lid(0));
    }
}
