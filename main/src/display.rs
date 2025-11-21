use crate::{
    container,
    container::Container,
    error::{Error, Result},
    status_bar::StatusBar,
    window::Window,
};
use core_graphics::{Bounds, Direction, DisplayId, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub(crate) struct VirtualDisplayId(pub usize);

impl std::fmt::Display for VirtualDisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for VirtualDisplayId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

pub(crate) struct PhysicalDisplay {
    bounds: Bounds,
    virtual_displays: HashMap<VirtualDisplayId, VirtualDisplay>,
    active_virtual_id: VirtualDisplayId,
    status_bar: StatusBar,
}

impl PhysicalDisplay {
    pub fn new(physical_id: DisplayId, cg_display: core_graphics::Display) -> Self {
        let mut virtual_display = VirtualDisplay::new(cg_display.bounds);
        for window in cg_display.windows {
            // TODO: handle
            let _ = virtual_display.add_window(window);
        }

        let virtual_id = VirtualDisplayId(usize::from(physical_id));

        let mut virtual_displays = HashMap::new();
        virtual_displays.insert(virtual_id, virtual_display);

        let status_bar = StatusBar::new(
            virtual_id,
            virtual_displays
                .keys()
                .cloned()
                .collect::<Vec<VirtualDisplayId>>(),
            cg_display.bounds,
        );

        status_bar.display();

        Self {
            bounds: cg_display.bounds,
            virtual_displays,
            active_virtual_id: virtual_id,
            status_bar,
        }
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        let mut all_window_ids: HashSet<WindowId> = HashSet::new();

        for vd in self.virtual_displays.values() {
            all_window_ids.extend(vd.window_ids());
        }

        all_window_ids
    }

    // When adding a window to a physical display, delegate to the currently
    // active virtual display.
    pub fn add_window(&mut self, cg_window: core_graphics::Window) -> Result<()> {
        // TODO: no unwrap
        self.virtual_displays
            .get_mut(&self.active_virtual_id)
            .unwrap()
            .add_window(cg_window)
    }

    // When removing a window from a physical display, delegate to the currently
    // active virtual display.
    pub fn remove_window(&mut self, window_id: WindowId) -> Result<bool> {
        self.virtual_displays
            .get_mut(&self.active_virtual_id)
            .unwrap()
            .remove_window(window_id)
    }

    pub fn contains_window(&self, window_id: WindowId) -> bool {
        match self.virtual_displays.get(&self.active_virtual_id) {
            None => false,
            Some(virtual_display) => virtual_display.window_ids().contains(&window_id),
        }
    }

    pub fn split(&mut self, direction: container::Direction) -> Result<()> {
        self.virtual_displays
            .get_mut(&self.active_virtual_id)
            .unwrap()
            .split(direction)
    }

    pub fn has_virtual_display(&self, virtual_id: VirtualDisplayId) -> bool {
        self.virtual_displays.contains_key(&virtual_id)
    }

    pub fn create_virtual_display(&mut self, virtual_id: VirtualDisplayId) {
        self.virtual_displays
            .insert(virtual_id, VirtualDisplay::new(self.bounds));
        self.update_status_bar();
    }

    pub fn add_window_to_virtual(
        &mut self,
        cg_window: core_graphics::Window,
        virtual_display_id: VirtualDisplayId,
    ) -> Result<()> {
        self.virtual_displays
            .get_mut(&virtual_display_id)
            .unwrap()
            .add_window(cg_window)
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.virtual_displays
            .get_mut(&self.active_virtual_id)
            .unwrap()
            .resize_focused_window(direction)
    }

    pub fn active_virtual_display(&self) -> Option<&VirtualDisplay> {
        self.virtual_displays.get(&self.active_virtual_id)
    }

    pub(crate) fn active_display(&self) -> &VirtualDisplay {
        // TODO: unwrap
        self.virtual_displays.get(&self.active_virtual_id).unwrap()
    }

    // Switching virtual display is done with the following steps:
    //  - If the target virtual display id is already active, do nothing
    //  - Else:
    //    1. Minimise all windows on the current virtual display
    //    2. Un-minimise all windows on the target virtual display id
    //    3. Focus the target virtual display
    //    4. If the previous virtual display now has no windows, delete it
    //    5. Update the physical display's status bar
    pub fn switch_to(&mut self, virtual_id: VirtualDisplayId) -> Result<()> {
        if virtual_id == self.active_virtual_id {
            return Ok(());
        }

        let current_virtual = self.virtual_displays.get(&self.active_virtual_id).unwrap();
        let new_virtual = self.virtual_displays.get(&virtual_id).unwrap();

        for window_id in current_virtual.window_ids() {
            if let Some(window) = current_virtual.find_window(window_id) {
                let _ = window.ax().minimise();
            }
        }

        for window_id in new_virtual.window_ids() {
            if let Some(window) = new_virtual.find_window(window_id) {
                let _ = window.ax().unminimise();
            }
        }

        // Focus the new virtual display if there are any windows on it
        if !new_virtual.window_ids().is_empty() {
            new_virtual.refocus()?;
        }

        // Remove the virtual window if there are no windows left
        if current_virtual.window_ids().is_empty() {
            self.virtual_displays.remove(&self.active_virtual_id);
        }

        self.active_virtual_id = virtual_id;

        self.update_status_bar();

        Ok(())
    }

    fn update_status_bar(&mut self) {
        self.status_bar.close();
        self.status_bar = StatusBar::new(
            self.active_virtual_id,
            self.virtual_displays
                .keys()
                .cloned()
                .collect::<Vec<VirtualDisplayId>>(),
            self.bounds,
        );
        self.status_bar.display();
    }

    pub fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        self.virtual_displays
            .get(&self.active_virtual_id)
            .unwrap()
            .find_window(window_id)
    }

    // Delegate focus shifting to the currently active virtual display.
    pub fn handle_focus_shift(&mut self, direction: Direction) -> Result<()> {
        self.virtual_displays
            .get_mut(&self.active_virtual_id)
            .ok_or(Error::DisplayNotFound)?
            .shift_focus(direction)
    }

    pub fn focus(&self) -> Result<()> {
        self.virtual_displays
            .get(&self.active_virtual_id)
            .unwrap()
            .refocus()
    }
}

#[derive(Debug)]
pub(crate) struct VirtualDisplay {
    root: Container,
    focused_window: Option<WindowId>,
}

impl VirtualDisplay {
    pub(crate) fn new(cg_bounds: Bounds) -> Self {
        // Core Graphics bounds -- the bounds used for a `PhysicalDisplay` do
        // not include the Apple menu bar; we need to subtract the height of
        // said bar to stop vertical split windows from overlapping each other.
        const MENU_BAR_HEIGHT: f64 = 37.0;

        // We also subtract the height of the i3-style `StatusBar` added to the
        // bottom of the screen.
        let bounds = Bounds {
            height: cg_bounds.height - MENU_BAR_HEIGHT - StatusBar::HEIGHT,
            y: cg_bounds.y + MENU_BAR_HEIGHT,
            ..cg_bounds
        };

        VirtualDisplay {
            root: Container::Empty { bounds },
            focused_window: None,
        }
    }

    // In order to switch focus in some direction:
    //  - Create a list of all window, sorted by either x or y position, based
    //    on the given direction.
    //  - If there are no windows at all (including the one that should be
    //    currently focused), return some error.
    //  - Find the focused window in this list.
    //  - If there are no more windows in the direction of the shift, return.
    //  - If there is one, focus it and return.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<()> {
        let mut all_windows = self.root.all_windows();
        match direction {
            Direction::Left | Direction::Right => {
                all_windows.sort_by(|a, b| a.bounds().x.total_cmp(&b.bounds().x));
                // all_windows.dedup_by(|a, b| a.bounds().x == b.bounds().x);
            }
            Direction::Up | Direction::Down => {
                all_windows.sort_by(|a, b| a.bounds().y.total_cmp(&b.bounds().y));
                // all_windows.dedup_by(|a, b| a.bounds().y == b.bounds().y);
            }
        }

        // TODO: handle panics
        let index_of_focused = all_windows
            .iter()
            .position(|window| window.cg().number() == self.focused_window.unwrap())
            .unwrap();

        match (index_of_focused, direction) {
            (n, Direction::Left | Direction::Up) if n != 0 => {
                self.focused_window = Some(all_windows[n - 1].cg().number());
            }
            (n, Direction::Right | Direction::Down) if n != all_windows.len() - 1 => {
                self.focused_window = Some(all_windows[n + 1].cg().number());
            }
            // We cannot move any further towards the edge of the screen, so do
            // nothing and return.
            _ => {}
        }

        self.refocus()
    }

    // When re-focusing a virtual display, focus the previously focused window.
    // If it does not exist, focus the first window found searching via BFS from
    // the root.
    pub fn refocus(&self) -> Result<()> {
        let to_focus = self
            .focused_window
            .or_else(|| self.window_ids().iter().next().copied())
            .ok_or(Error::CannotFocusEmptyDisplay)?;

        if let Some(window) = self.root.find_window(to_focus) {
            window.ax().try_focus().map_err(Error::AxUi)
        } else {
            Err(Error::WindowNotFound)
        }
    }

    pub(crate) fn window_ids(&self) -> HashSet<WindowId> {
        self.root.window_ids()
    }

    pub fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        self.root.find_window(window_id)
    }

    // When splitting a virtual display in some direction:
    //  1. If there is some focused window, convert its parent leaf into a split
    //     of the given direction, and shift the leaf into it.
    //  2. If there is no focused window, switch the split direction of the root
    //     container.
    pub fn split(&mut self, direction: container::Direction) -> Result<()> {
        if let Some(window_id) = self.focused_window {
            let container = self
                .root
                .parent_leaf_of_window_mut(window_id)
                .ok_or(Error::CannotFindParentLeaf)?;
            container.split(direction)
        } else {
            self.root.split(direction)
        }
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<bool> {
        let removed = self.root.remove_window(window_id)?;
        if self.focused_window == Some(window_id) {
            self.focused_window = self.window_ids().iter().next().copied();
        }
        Ok(removed)
    }

    // When adding a window to a virtual display, see if there is a previously
    // focused window.
    // If so:
    //  - Find the split that owns the window
    //  - Add new window as a child to that split
    // If there is no window:
    //  - Add new window as a child of the root (horizontal split)
    pub fn add_window(&mut self, window: core_graphics::Window) -> Result<()> {
        let window_id = window.number();

        let container = if let Some(focused_id) = self.focused_window
            && let Some(container) = self.root.get_parent_of_window_mut(focused_id)
        {
            container
        } else {
            &mut self.root
        };

        self.focused_window = Some(window_id);
        container.add_window(window)
    }

    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        if let Some(focused_id) = self.focused_window {
            self.resize_window_in_direction(focused_id, direction)?;
        }

        Ok(())
    }

    pub fn resize_window_in_direction(
        &mut self,
        window_id: WindowId,
        direction: Direction,
    ) -> Result<()> {
        const RESIZE_AMOUNT: f64 = 50.0;
        self.root.resize_window(window_id, direction, RESIZE_AMOUNT)
    }
}
