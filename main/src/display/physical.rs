use crate::{
    container,
    display::{LogicalDisplay, LogicalDisplayId},
    error::{Error, Result},
    status_bar::StatusBar,
};
use container::Axis;
use core_graphics::{Bounds, Direction, DisplayId, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Copy, Clone)]
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
    status_bar: StatusBar,
    config: Config,
}

impl PhysicalDisplay {
    pub fn new(physical_id: DisplayId, cg_display: core_graphics::Display, config: Config) -> Self {
        let mut logical_display = LogicalDisplay::new(cg_display.bounds, config.into());
        for window in cg_display.windows {
            // TODO: handle
            let _ = logical_display.add_window(window);
        }

        let logical_id = LogicalDisplayId(usize::from(physical_id));

        let mut logical_displays = HashMap::new();
        logical_displays.insert(logical_id, logical_display);

        let status_bar = StatusBar::new(
            logical_id,
            logical_displays
                .keys()
                .cloned()
                .collect::<Vec<LogicalDisplayId>>(),
            cg_display.bounds,
        );

        status_bar.display();

        Self {
            bounds: cg_display.bounds,
            logical_displays,
            active_logical_id: logical_id,
            status_bar,
            config,
        }
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        let mut all_window_ids: HashSet<WindowId> = HashSet::new();

        for vd in self.logical_displays.values() {
            all_window_ids.extend(vd.window_ids());
        }

        all_window_ids
    }

    // When adding a window to a physical display, delegate to the currently
    // active logical display.
    pub fn add_window(&mut self, cg_window: core_graphics::Window) -> Result<()> {
        // TODO: no unwrap
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .unwrap()
            .add_window(cg_window)
    }

    // When removing a window from a physical display, delegate to the currently
    // active logical display.
    pub fn remove_window(&mut self, window_id: WindowId) -> Result<bool> {
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .unwrap()
            .remove_window(window_id)
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

    pub fn create_logical_display(&mut self, logical_id: LogicalDisplayId) {
        self.logical_displays.insert(
            logical_id,
            LogicalDisplay::new(self.bounds, self.config.into()),
        );
        self.update_status_bar();
    }

    pub fn add_window_to_logical(
        &mut self,
        cg_window: core_graphics::Window,
        logical_display_id: LogicalDisplayId,
    ) -> Result<()> {
        self.logical_displays
            .get_mut(&logical_display_id)
            .unwrap()
            .add_window(cg_window)
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
    pub fn switch_to(&mut self, logical_id: LogicalDisplayId) -> Result<()> {
        if logical_id == self.active_logical_id {
            return Ok(());
        }

        let current_logical = self.logical_displays.get(&self.active_logical_id).unwrap();
        let new_logical = self.logical_displays.get(&logical_id).unwrap();

        for window_id in current_logical.window_ids() {
            if let Some(window) = current_logical.find_window(window_id) {
                let _ = window.ax().minimise();
            }
        }

        for window_id in new_logical.window_ids() {
            if let Some(window) = new_logical.find_window(window_id) {
                let _ = window.ax().unminimise();
            }
        }

        // Focus the new logical display if there are any windows on it
        if !new_logical.window_ids().is_empty() {
            new_logical.refocus()?;
        }

        // Remove the logical window if there are no windows left
        if current_logical.window_ids().is_empty() {
            self.logical_displays.remove(&self.active_logical_id);
        }

        self.active_logical_id = logical_id;

        self.update_status_bar();

        Ok(())
    }

    fn update_status_bar(&mut self) {
        self.status_bar.close();
        self.status_bar = StatusBar::new(
            self.active_logical_id,
            self.logical_displays
                .keys()
                .cloned()
                .collect::<Vec<LogicalDisplayId>>(),
            self.bounds,
        );
        self.status_bar.display();
    }

    // Delegate focus shifting to the currently active logical display.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<()> {
        self.logical_displays
            .get_mut(&self.active_logical_id)
            .ok_or(Error::DisplayNotFound)?
            .shift_focus(direction)
    }

    pub fn focus(&self) -> Result<()> {
        self.logical_displays
            .get(&self.active_logical_id)
            .unwrap()
            .refocus()
    }
}
