mod logical;
mod physical;
mod tests;

use crate::container;
use crate::container::Axis;
use crate::error::Error;
use crate::error::Result;
use container::Window;
use core_graphics::{Bounds, WindowId};
pub use logical::LogicalDisplayId;
use logical::*;
pub use physical::PhysicalDisplayId;
use physical::*;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Displays {
    physical_displays: HashMap<PhysicalDisplayId, PhysicalDisplay>,
    active_logical_display_ids: HashMap<LogicalDisplayId, PhysicalDisplayId>,
    active_physical_display_id: Option<PhysicalDisplayId>,
}

impl Displays {
    pub fn display_of_window(&self, wid: WindowId) -> Option<PhysicalDisplayId> {
        self.physical_displays
            .iter()
            .find(|(_, pd)| pd.window_ids().contains(&wid))
            .map(|(pid, _)| *pid)
    }

    pub fn focus_display(&mut self, lid: LogicalDisplayId) -> WindowId {
        let pid = *self.active_logical_display_ids.get(&lid).unwrap();
        self.active_physical_display_id = Some(pid);
        self.physical_displays
            .get(&pid)
            .unwrap()
            .focused_window()
            .unwrap()
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.physical_displays
            .get_mut(&self.active_physical_display_id.unwrap())
            .unwrap()
            .split(axis)
    }

    pub fn set_active_physical_display(&mut self, id: PhysicalDisplayId) {
        self.active_physical_display_id = Some(id);
    }

    pub fn physical_displays(&self) -> &HashMap<PhysicalDisplayId, PhysicalDisplay> {
        &self.physical_displays
    }

    pub fn switch_logical_display(
        &mut self,
        pid: PhysicalDisplayId,
        new_lid: LogicalDisplayId,
    ) -> Result<Option<LogicalDisplayId>> {
        let old_lid = self
            .physical_displays
            .get(&pid)
            .unwrap()
            .active_logical_id();
        let removed = self
            .physical_displays
            .get_mut(&pid)
            .unwrap()
            .switch_to(new_lid)?;
        if removed {
            self.active_logical_display_ids.remove(&old_lid);
            return Ok(Some(old_lid));
        }
        Ok(None)
    }

    pub fn next_logical_display_id(&mut self, pid: PhysicalDisplayId) -> Option<LogicalDisplayId> {
        for id in 0..=9 {
            let lid = LogicalDisplayId(id);
            if let std::collections::hash_map::Entry::Vacant(e) =
                self.active_logical_display_ids.entry(lid)
            {
                e.insert(pid);
                return Some(lid);
            }
        }

        None
    }

    pub fn add_physical(&mut self, pid: PhysicalDisplayId, bounds: Bounds, cfg: physical::Config) {
        let next_logical_id = self
            .next_logical_display_id(pid)
            .expect("already have 10 logical displays");

        let pd = PhysicalDisplay::new(next_logical_id, bounds, cfg);

        self.physical_displays.insert(pid, pd);
        self.active_logical_display_ids.insert(next_logical_id, pid);

        self.active_physical_display_id = Some(pid);
    }

    pub(crate) fn active_logical_display_id(&self) -> LogicalDisplayId {
        self.physical_displays
            .get(&self.active_physical_display_id.unwrap())
            .unwrap()
            .active_logical_id()
    }

    pub fn create_logical_display(
        &mut self,
        pid: PhysicalDisplayId,
        lid: LogicalDisplayId,
    ) -> LogicalDisplayId {
        assert!(!self.active_logical_display_ids.contains_key(&lid));

        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .create_logical_display(lid);

        self.active_logical_display_ids.insert(lid, pid);

        lid
    }

    pub fn physical_display_mut(&mut self, pid: PhysicalDisplayId) -> Option<&mut PhysicalDisplay> {
        self.physical_displays.get_mut(&pid)
    }

    pub fn add_window(&mut self, window: Window) -> Result<AddWindowResult> {
        let initial_lid = self.active_logical_display_id();
        let mut lid = initial_lid;
        let pid = *self.active_logical_display_ids.get(&lid).unwrap();

        while let Err(e) = self
            .physical_displays
            .get_mut(&pid)
            .unwrap()
            .add_window_to_logical(window, lid)
        {
            if e == Error::CannotFitWindow {
                lid = self.next_logical_display_id(pid).unwrap();
                self.physical_displays
                    .get_mut(&pid)
                    .unwrap()
                    .create_logical_display(lid);
                self.active_logical_display_ids.insert(lid, pid);
            } else {
                return Err(e);
            }
        }

        if lid == initial_lid {
            Ok(AddWindowResult::Active(lid))
        } else {
            Ok(AddWindowResult::Overflow(lid))
        }
    }

    pub fn add_window_to_logical(&mut self, window: Window, lid: LogicalDisplayId) -> Result<()> {
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

    pub fn remove_window(
        &mut self,
        pid: PhysicalDisplayId,
        wid: WindowId,
    ) -> Result<Option<WindowId>> {
        self.physical_displays
            .get_mut(&pid)
            .unwrap()
            .remove_window(wid)
    }

    pub fn logical_ids(&self, pid: PhysicalDisplayId) -> HashSet<LogicalDisplayId> {
        self.active_logical_display_ids
            .iter()
            .filter_map(|(l, p)| if *p == pid { Some(*l) } else { None })
            .collect()
    }

    pub fn active_physical_display_mut(&mut self) -> &mut PhysicalDisplay {
        self.physical_displays
            .get_mut(&self.active_physical_display_id.unwrap())
            .unwrap()
    }
}

pub enum AddWindowResult {
    Active(LogicalDisplayId),
    Overflow(LogicalDisplayId),
}

#[cfg(test)]
mod test {
    use super::*;

    fn pid(id: usize) -> PhysicalDisplayId {
        PhysicalDisplayId(id)
    }

    fn lid(id: usize) -> LogicalDisplayId {
        LogicalDisplayId(id)
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
