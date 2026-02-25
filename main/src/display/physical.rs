use crate::{
    container,
    display::{LogicalDisplay, LogicalDisplayId},
    error::{Error, Result},
};
use container::{Axis, Window};
use core_graphics::{Bounds, Direction, DisplayId, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub struct PhysicalDisplayId(pub usize);

impl std::fmt::Display for PhysicalDisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PD{}", self.0)
    }
}

impl From<DisplayId> for PhysicalDisplayId {
    fn from(id: DisplayId) -> Self {
        Self(usize::from(id))
    }
}

impl From<PhysicalDisplayId> for DisplayId {
    fn from(id: PhysicalDisplayId) -> Self {
        DisplayId::from(id.0)
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    pub window_padding: Option<f64>,
}

impl From<crate::window_manager::Config> for Config {
    fn from(value: crate::window_manager::Config) -> Self {
        Self {
            window_padding: value.window_padding,
        }
    }
}

pub(crate) struct PhysicalDisplay {
    bounds: Bounds,
    logical_displays: HashMap<LogicalDisplayId, LogicalDisplay>,
    active_logical_id: LogicalDisplayId,
    config: Config,
}

impl PhysicalDisplay {
    pub fn new(logical_id: LogicalDisplayId, bounds: Bounds, config: Config) -> Self {
        let logical_display = LogicalDisplay::new(bounds, config.into());
        // for window in cg_display.windows {
        //     // TODO: handle
        //     let cw = container::Window{
        //         id: window.number(),
        //         min_width: window.,
        //         min_height: 0.0,
        //     }
        //     let _ = logical_display.add_window(window.number());
        // }

        let mut logical_displays = HashMap::new();
        logical_displays.insert(logical_id, logical_display);

        Self {
            bounds,
            logical_displays,
            active_logical_id: logical_id,
            config,
        }
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) {
        if let Some(lid) = self.logical_displays.get_mut(&self.active_logical_id) {
            lid.set_focused_window(window_id);
        }
    }

    pub fn active_window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.logical_displays
            .get(&self.active_logical_id)
            .map(|ld| ld.window_bounds())
            .unwrap_or_default()
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        let mut all_window_ids: HashSet<WindowId> = HashSet::new();

        for vd in self.logical_displays.values() {
            all_window_ids.extend(vd.window_ids());
        }

        all_window_ids
    }

    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.logical_displays
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
        // TODO: no unwrap
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .unwrap()
            .add_window(window)
    }

    pub fn active_logical_id(&self) -> LogicalDisplayId {
        self.active_logical_id
    }

    // When removing a window from a physical display, delegate to the currently
    // active logical display.
    pub fn remove_window(&mut self, window_id: WindowId) -> Result<Option<WindowId>> {
        let owner = self
            .logical_displays
            .values_mut()
            .find(|ld| ld.window_ids().contains(&window_id))
            .ok_or(Error::WindowNotFound)?;
        owner.remove_window(window_id)
    }

    pub fn split(&mut self, direction: Axis) -> Result<()> {
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .unwrap()
            .split(direction)
    }

    pub fn has_logical_display(&self, logical_id: LogicalDisplayId) -> bool {
        self.logical_displays.contains_key(&logical_id)
    }

    pub(super) fn create_logical_display(&mut self, logical_id: LogicalDisplayId) {
        self.logical_displays.insert(
            logical_id,
            LogicalDisplay::new(self.bounds, self.config.into()),
        );
    }

    pub fn remove_logical_display(&mut self, logical_id: LogicalDisplayId) {
        // TODO: error trying to remove last one
        self.logical_displays.remove(&logical_id);

        // Crude way of getting new active LD
        if let Some(k) = self.logical_displays.keys().next() {
            self.active_logical_id = *k;
        }
    }

    pub fn add_window_to_logical(
        &mut self,
        window: Window,
        logical_display_id: LogicalDisplayId,
    ) -> Result<()> {
        self.logical_displays
            .get_mut(&logical_display_id)
            .unwrap()
            .add_window(window)
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .unwrap()
            .resize_focused_window(direction)
    }

    pub fn active_logical_display(&self) -> Option<&LogicalDisplay> {
        self.logical_displays.get(&self.active_logical_id)
    }

    pub(crate) fn active_display(&self) -> &LogicalDisplay {
        // TODO: unwrap
        self.logical_displays.get(&self.active_logical_id).unwrap()
    }

    // Switching logical display is done with the following steps:
    //  - If the target logical display id is already active, do nothing
    //  - Else:
    //    1. Minimise all windows on the current logical display
    //    2. Un-minimise all windows on the target logical display id
    //    3. Focus the target logical display
    //    4. If the previous logical display now has no windows, delete it
    //    5. Update the physical display's status bar
    pub fn switch_to(&mut self, logical_id: LogicalDisplayId) -> Result<bool> {
        if logical_id == self.active_logical_id {
            return Ok(false);
        }

        let current_logical = self.logical_displays.get(&self.active_logical_id).unwrap();

        let mut removed = false;
        // Remove the logical display if there are no windows left
        if current_logical.window_ids().is_empty() {
            self.logical_displays.remove(&self.active_logical_id);
            removed = true;
        }

        self.active_logical_id = logical_id;

        Ok(removed)
    }

    // Delegate focus shifting to the currently active logical display.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .ok_or(Error::DisplayNotFound)?
            .shift_focus(direction)
    }
}
