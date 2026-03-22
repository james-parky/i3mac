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
use core_graphics::{Bounds, Direction, DisplayId, WindowId};
use std::collections::{HashMap, HashSet};

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

pub(crate) struct Display {
    bounds: Bounds,
    empty_logicals: HashMap<logical::Id, logical::Display<NoWindows>>,
    occupied_logicals: HashMap<logical::Id, logical::Display<SomeWindows>>,
    active_logical_id: logical::Id,
    config: Config,
    logger: Logger,
}

impl Display {
    pub fn new(physical_id: Id, logical_id: logical::Id, bounds: Bounds, config: Config) -> Self {
        let logical_display = logical::Display::new(logical_id, bounds, config.into());

        let mut logical_displays = HashMap::new();
        logical_displays.insert(logical_id, logical_display);

        let mut logger =
            Logger::try_new("/dev/stdout", config.log_level, physical_id.as_log_prefix()).unwrap();

        PhysicalNew.log(&mut logger);
        Self {
            bounds,
            empty_logicals: logical_displays,
            occupied_logicals: HashMap::new(),
            active_logical_id: logical_id,
            config,
            logger,
        }
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) {
        for ld in self.occupied_logicals.values_mut() {
            if ld.window_ids().contains(&window_id) {
                ld.set_focused_window(window_id).unwrap();
                PhysicalSetFocused(window_id).log(&mut self.logger);
                return;
            }
        }

        // TODO: return some error or panic
    }

    pub fn active_window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.occupied_logicals
            .get(&self.active_logical_id)
            .map(|ld| ld.window_bounds())
            .unwrap_or_default()
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        let mut all_window_ids: HashSet<WindowId> = HashSet::new();

        for ld in self.occupied_logicals.values() {
            all_window_ids.extend(ld.window_ids());
        }

        all_window_ids
    }

    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.occupied_logicals
            .values()
            .flat_map(|d| d.window_bounds())
            .collect()
    }

    // When adding a window to a physical display, delegate to the currently
    // active logical display.
    //
    // Either the window will be added to the physical display's active logical
    // display, or a new logical display will be created and made active for it.
    pub fn add_window(&mut self, window: Window) -> Result<()> {
        let lid = self.active_logical_id;

        if let Some(empty) = self.empty_logicals.remove(&lid) {
            let occupied = empty.add_window(window)?;
            self.occupied_logicals.insert(lid, occupied);
        } else if let Some(occupied) = self.occupied_logicals.get_mut(&lid) {
            occupied.add_window(window)?;
        } else {
            return Err(Error::DisplayNotFound);
        }

        PhysicalAddedWindow(window.id).log(&mut self.logger);
        Ok(())
    }

    pub fn active_logical_id(&self) -> logical::Id {
        self.active_logical_id
    }

    pub fn logical_is_empty(&self, id: logical::Id) -> bool {
        self.empty_logicals.contains_key(&id)
    }

    pub fn occupied_logical(&self, id: logical::Id) -> Option<&logical::Display<SomeWindows>> {
        self.occupied_logicals.get(&id)
    }

    // When removing a window from a physical display, delegate to the currently
    // active logical display.
    pub fn remove_window(&mut self, window_id: WindowId) -> Result<()> {
        let lid = self
            .occupied_logicals
            .iter()
            .find(|(_, ld)| ld.window_ids().contains(&window_id))
            .map(|(lid, _)| *lid)
            .ok_or(Error::WindowNotFound)?;

        // Safety: we just confirmed it is in the set of logical displays
        let occupied = self.occupied_logicals.remove(&lid).unwrap();

        match occupied.remove_window(window_id)? {
            logical::RemoveResult::NowEmpty(display) => {
                self.empty_logicals.insert(lid, display);

                if lid == self.active_logical_id {
                    if let Some(&new) = self.occupied_logicals.keys().next() {
                        self.active_logical_id = new;
                    }
                }
            }
            logical::RemoveResult::StillHasWindows(display) => {
                self.occupied_logicals.insert(lid, display);
            }
        }

        PhysicalRemovedWindow(window_id).log(&mut self.logger);
        Ok(())
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.occupied_logicals
            .get_mut(&self.active_logical_id)
            .ok_or(Error::CannotSplitEmptyContainer)?
            .split(axis)?;

        PhysicalSplit(axis).log(&mut self.logger);
        Ok(())
    }

    pub fn has_logical_display(&self, id: logical::Id) -> bool {
        self.empty_logicals.contains_key(&id) || self.occupied_logicals.contains_key(&id)
    }

    pub(super) fn create_logical_display(&mut self, id: logical::Id) {
        let ld = logical::Display::new(id, self.bounds, self.config.into());
        self.empty_logicals.insert(id, ld);
        PhysicalAddedLogical(id).log(&mut self.logger);
    }

    pub fn remove_logical_display(&mut self, id: logical::Id) {
        // Can only remove if its empty? TODO: Check this returned Some
        self.empty_logicals.remove(&id);
        PhysicalRemovedLogical(id).log(&mut self.logger);

        // Crude way of getting new active LD
        if let Some(k) = self.occupied_logicals.keys().next() {
            self.active_logical_id = *k;
            PhysicalSwitchActive(*k).log(&mut self.logger);
        }
    }

    pub fn add_window_to_logical(&mut self, window: Window, id: logical::Id) -> Result<()> {
        if let Some(empty) = self.empty_logicals.remove(&id) {
            let occupied = empty.add_window(window)?;
            self.occupied_logicals.insert(id, occupied);
        } else if let Some(occupied) = self.occupied_logicals.get_mut(&id) {
            occupied.add_window(window)?;
        } else {
            return Err(Error::DisplayNotFound);
        }

        PhysicalAddedWindowToLogical(window.id, id).log(&mut self.logger);
        Ok(())
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.occupied_logicals
            .get_mut(&self.active_logical_id)
            .unwrap()
            .resize_focused_window(direction)?;

        PhysicalResizeFocused(direction).log(&mut self.logger);
        Ok(())
    }

    // pub fn active_logical_display(&self) -> Option<&logical::Display> {
    //     self.logical_displays.get(&self.active_logical_id)
    // }
    //
    // pub(crate) fn active_display(&self) -> &logical::Display {
    //     // TODO: unwrap
    //     self.logical_displays.get(&self.active_logical_id).unwrap()
    // }

    // Switching logical display is done with the following steps:
    //  - If the target logical display id is already active, do nothing
    //  - Else:
    //    1. Minimise all windows on the current logical display
    //    2. Un-minimise all windows on the target logical display id
    //    3. Focus the target logical display
    //    4. If the previous logical display now has no windows, delete it
    //    5. Update the physical display's status bar
    pub fn switch_to(&mut self, id: logical::Id) {
        if id != self.active_logical_id {
            self.active_logical_id = id;
        }

        PhysicalSwitchDisplay(id).log(&mut self.logger);
    }

    // Delegate focus shifting to the currently active logical display.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        let window = self
            .occupied_logicals
            .get_mut(&self.active_logical_id)
            .ok_or(Error::DisplayNotFound)?
            .shift_focus(direction)?;

        PhysicalShiftFocus(direction, window).log(&mut self.logger);
        Ok(window)
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.occupied_logicals
            .get(&self.active_logical_id)
            .map(|l| l.focused_window())
    }
}
