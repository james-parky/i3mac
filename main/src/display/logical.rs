use crate::{
    container,
    container::Container,
    error::{Error, Result},
    status_bar::StatusBar,
    window::Window,
};
use container::Axis;
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub(crate) struct LogicalDisplayId(pub usize);

impl std::fmt::Display for LogicalDisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for LogicalDisplayId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub(crate) struct LogicalDisplay {
    root: Container,
    focused_window: Option<WindowId>,
}

impl LogicalDisplay {
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

        LogicalDisplay {
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

    // When re-focusing a logical display, focus the previously focused window.
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

    // When splitting a logical display in some direction:
    //  1. If there is some focused window, convert its parent leaf into a split
    //     of the given direction, and shift the leaf into it.
    //  2. If there is no focused window, switch the split direction of the root
    //     container.
    pub fn split(&mut self, direction: Axis) -> Result<()> {
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

    // When adding a window to a logical display, see if there is a previously
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
