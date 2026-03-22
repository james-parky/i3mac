use crate::display::logical::{NoWindows, SomeWindows};
use crate::log::Prefix;
use crate::{
    container::{Axis, Window},
    display::{
        log::Message::{
            PhysicalAddedLogical, PhysicalAddedWindow, PhysicalAddedWindowToLogical, PhysicalNew,
            PhysicalRemovedLogical, PhysicalRemovedWindow, PhysicalResizeFocused,
            PhysicalSetFocused, PhysicalShiftFocus, PhysicalSplit, PhysicalSwitchActive,
            PhysicalSwitchDisplay,
        },
        logical,
    },
    error::{Error, Result},
    log::{Level, Log, Logger},
};
use core_graphics::{Bounds, Direction, DisplayId, Identity, WindowId};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub struct Id(pub usize);

// TODO: Display for physical::Id has the PD prefix, but this is Debug for logical::Id.
impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PD{}", self.0)
    }
}

impl From<DisplayId> for Id {
    fn from(id: DisplayId) -> Self {
        Self(usize::from(id))
    }
}

impl From<Id> for DisplayId {
    fn from(id: Id) -> Self {
        DisplayId::from(id.0)
    }
}

impl Id {
    fn as_log_prefix(&self) -> Prefix {
        Prefix::new(format!("PD{} ", self.0))
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    pub window_padding: Option<f64>,
    pub log_level: Level,
}

impl From<crate::config::Config> for Config {
    fn from(value: crate::config::Config) -> Self {
        Self {
            window_padding: value.window_padding,
            log_level: value.log_level,
        }
    }
}

pub struct LogicalDisplays {
    empty: HashMap<logical::Id, logical::Display<NoWindows>>,
    occupied: HashMap<logical::Id, logical::Display<SomeWindows>>,
    active: logical::Id,
}

impl LogicalDisplays {
    pub fn new(id: logical::Id, display: logical::Display<NoWindows>) -> Self {
        let mut empty = HashMap::new();
        empty.insert(id, display);

        Self {
            empty,
            occupied: HashMap::new(),
            active: id,
        }
    }

    pub fn contains(&self, id: logical::Id) -> bool {
        self.occupied.contains_key(&id) || self.empty.contains_key(&id)
    }

    pub fn is_empty(&self, id: logical::Id) -> bool {
        self.empty.contains_key(&id)
    }

    pub fn get_empty(&self, id: logical::Id) -> Option<&logical::Display<NoWindows>> {
        self.empty.get(&id)
    }

    pub fn get_occupied(&self, id: logical::Id) -> Option<&logical::Display<SomeWindows>> {
        self.occupied.get(&id)
    }

    pub fn get_occupied_mut(
        &mut self,
        id: logical::Id,
    ) -> Option<&mut logical::Display<SomeWindows>> {
        self.occupied.get_mut(&id)
    }

    pub fn occupied_ids(&self) -> impl Iterator<Item = logical::Id> + '_ {
        self.occupied.keys().copied()
    }

    pub fn all_ids(&self) -> impl Iterator<Item = logical::Id> + '_ {
        self.empty.keys().chain(self.occupied.keys()).copied()
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        self.occupied
            .values()
            .flat_map(|d| d.window_ids())
            .collect()
    }

    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.occupied
            .values()
            .flat_map(|d| d.window_bounds())
            .collect()
    }

    pub fn insert_empty(&mut self, id: logical::Id, display: logical::Display<NoWindows>) {
        self.empty.insert(id, display);
    }

    pub fn add_window(&mut self, id: logical::Id, window: Window) -> Result<()> {
        if let Some(empty) = self.empty.remove(&id) {
            let occupied = empty.add_window(window)?;
            self.occupied.insert(id, occupied);
        } else if let Some(occupied) = self.occupied.get_mut(&id) {
            occupied.add_window(window)?;
        } else {
            return Err(Error::DisplayNotFound);
        }

        Ok(())
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<()> {
        let lid = self
            .occupied
            .iter()
            .find(|(_, ld)| ld.window_ids().contains(&window_id))
            .map(|(lid, _)| *lid)
            .ok_or(Error::WindowNotFound)?;

        // Safety: we just confirmed it is in the set of logical displays
        let occupied = self.occupied.remove(&lid).unwrap();

        match occupied.remove_window(window_id)? {
            logical::RemoveResult::NowEmpty(display) => {
                self.empty.insert(lid, display);

                if lid == self.active {
                    if let Some(&new) = self.occupied.keys().next() {
                        self.active = new;
                    }
                }
            }
            logical::RemoveResult::StillHasWindows(display) => {
                self.occupied.insert(lid, display);
            }
        }

        Ok(())
    }

    pub fn remove_logical(&mut self, id: logical::Id) -> Result<()> {
        self.empty
            .remove(&id)
            .ok_or(Error::CannotRemoveOccupiedLogical)?;

        if self.active == id {
            // Safety: there must at least 1 display left, occupied or empty
            self.active = self
                .occupied
                .keys()
                .next()
                .or_else(|| self.empty.keys().next())
                .copied()
                .unwrap();
        }
        Ok(())
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.occupied
            .get_mut(&self.active)
            .ok_or(Error::CannotSplitEmptyLogical)?
            .split(axis)
    }

    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        self.occupied
            .get_mut(&self.active)
            .ok_or(Error::CannotFocusEmptyDisplay)?
            .shift_focus(direction)
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) -> Result<()> {
        for ld in self.occupied.values_mut() {
            if ld.window_ids().contains(&window_id) {
                ld.set_focused_window(window_id)?;
            }
        }

        Err(Error::CannotFindWindow)
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.occupied
            .get(&self.active)
            .map(|ld| ld.focused_window())
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.occupied
            .get_mut(&self.active)
            .ok_or(Error::CannotFocusEmptyDisplay)?
            .resize_focused_window(direction)
    }

    pub fn switch_to(&mut self, id: logical::Id) {
        if id != self.active {
            self.active = id;
        }
    }
}

pub(crate) struct Display {
    bounds: Bounds,
    logicals: LogicalDisplays,
    config: Config,
    logger: Logger,
}

impl Display {
    pub fn new(physical_id: Id, logical_id: logical::Id, bounds: Bounds, config: Config) -> Self {
        let logical_display = logical::Display::new(logical_id, bounds, config.into());

        let mut logger =
            Logger::try_new("/dev/stdout", config.log_level, physical_id.as_log_prefix()).unwrap();

        PhysicalNew.log(&mut logger);
        Self {
            bounds,
            logicals: LogicalDisplays::new(logical_id, logical_display),
            config,
            logger,
        }
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) -> Result<()> {
        self.logicals.set_focused_window(window_id)
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        self.logicals.window_ids()
    }

    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.logicals.window_bounds()
    }

    // When adding a window to a physical display, delegate to the currently
    // active logical display.
    //
    // Either the window will be added to the physical display's active logical
    // display, or a new logical display will be created and made active for it.
    pub fn add_window(&mut self, window: Window) -> Result<()> {
        self.logicals.add_window(self.logicals.active, window)?;
        PhysicalAddedWindow(window.id).log(&mut self.logger);
        Ok(())
    }

    pub fn active_logical_id(&self) -> logical::Id {
        self.logicals.active
    }

    pub fn logical_is_empty(&self, id: logical::Id) -> bool {
        self.logicals.is_empty(id)
    }

    pub fn occupied_logical(&self, id: logical::Id) -> Option<&logical::Display<SomeWindows>> {
        self.logicals.get_occupied(id)
    }

    // When removing a window from a physical display, delegate to the currently
    // active logical display.
    pub fn remove_window(&mut self, window_id: WindowId) -> Result<()> {
        self.logicals.remove_window(window_id)?;
        PhysicalRemovedWindow(window_id).log(&mut self.logger);
        Ok(())
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.logicals.split(axis)?;
        PhysicalSplit(axis).log(&mut self.logger);
        Ok(())
    }

    pub fn has_logical_display(&self, id: logical::Id) -> bool {
        self.logicals.contains(id)
    }

    pub(super) fn create_logical_display(&mut self, id: logical::Id) {
        let ld = logical::Display::new(id, self.bounds, self.config.into());
        self.logicals.insert_empty(id, ld);
        PhysicalAddedLogical(id).log(&mut self.logger);
    }

    pub fn remove_logical_display(&mut self, id: logical::Id) -> Result<()> {
        self.logicals.remove_logical(id)?;
        PhysicalRemovedLogical(id).log(&mut self.logger);
        Ok(())
    }

    pub fn add_window_to_logical(&mut self, window: Window, id: logical::Id) -> Result<()> {
        self.logicals.add_window(id, window)?;
        PhysicalAddedWindowToLogical(window.id, id).log(&mut self.logger);
        Ok(())
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.logicals.resize_focused_window(direction)?;
        PhysicalResizeFocused(direction).log(&mut self.logger);
        Ok(())
    }

    pub fn switch_to(&mut self, id: logical::Id) {
        self.logicals.switch_to(id);
        PhysicalSwitchDisplay(id).log(&mut self.logger);
    }

    // Delegate focus shifting to the currently active logical display.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        let window = self.logicals.shift_focus(direction)?;
        PhysicalShiftFocus(direction, window).log(&mut self.logger);
        Ok(window)
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.logicals.focused_window()
    }
}
