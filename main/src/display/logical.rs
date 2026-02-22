use crate::{
    container,
    container::{Axis, Container},
    error::{Error, Result},
    status_bar::StatusBar,
};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

#[derive(PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub struct LogicalDisplayId(pub usize);

impl std::fmt::Display for LogicalDisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for LogicalDisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "LD{}", self.0)
    }
}

impl From<usize> for LogicalDisplayId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Config {
    window_padding: Option<f64>,
}

impl From<crate::display::physical::Config> for Config {
    fn from(config: crate::display::physical::Config) -> Self {
        Self {
            window_padding: config.window_padding,
        }
    }
}

impl Config {
    pub fn window_padding(&self) -> f64 {
        self.window_padding.unwrap_or_default()
    }
}

#[derive(Debug)]
pub(crate) struct LogicalDisplay {
    root: Container,
    focused_window: Option<WindowId>,
    config: Config,
}

impl LogicalDisplay {
    pub(crate) fn new(cg_bounds: Bounds, config: Config) -> Self {
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
            config,
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
    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        let mut windows = self.all_window_bounds();
        if windows.is_empty() {
            return Err(Error::CannotFocusEmptyDisplay);
        }

        match direction {
            Direction::Left | Direction::Right => windows.sort_by(|a, b| a.1.x.total_cmp(&b.1.x)),
            Direction::Up | Direction::Down => windows.sort_by(|a, b| a.1.y.total_cmp(&b.1.y)),
        };

        let current_focused = self.focused_window.unwrap_or(windows[0].0);
        let current_focussed_index = windows
            .iter()
            .position(|(id, _)| *id == current_focused)
            .unwrap_or(0);

        let next_focus = match (current_focussed_index, direction) {
            (n, Direction::Left | Direction::Up) if n != 0 => windows[n - 1].0,
            (n, Direction::Right | Direction::Down) if n < windows.len() - 1 => windows[n + 1].0,
            _ => windows[current_focussed_index].0,
        };
        self.focused_window = Some(next_focus);
        Ok(next_focus)
    }

    fn all_window_bounds(&self) -> Vec<(WindowId, Bounds)> {
        fn recurse(c: &Container, out: &mut Vec<(WindowId, Bounds)>) {
            match c {
                Container::Leaf { window, .. } => {
                    if let Some(b) = c.window_bounds() {
                        out.push((window.id, b));
                    }
                }
                Container::Split { children, .. } => {
                    for child in children {
                        recurse(child, out);
                    }
                }
                Container::Empty { .. } => {}
            }
        }

        let mut result = Vec::new();
        recurse(&self.root, &mut result);
        result
    }

    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.root.window_bounds_by_id()
    }

    pub(crate) fn window_ids(&self) -> HashSet<WindowId> {
        self.root.window_ids()
    }

    pub fn find_window(&self, window_id: WindowId) -> Option<WindowId> {
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

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<Option<WindowId>> {
        let removed = self
            .root
            .remove_window(window_id, self.config.window_padding())?;

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
    pub fn add_window(&mut self, window: container::Window) -> Result<()> {
        println!("adding window {window:?} to {self:?}");
        let container = if let Some(focused_id) = self.focused_window
            && let Some(container) = self.root.get_parent_of_window_mut(focused_id)
        {
            container
        } else {
            &mut self.root
        };

        match container.add_window(window, self.config.window_padding()) {
            Ok(()) => {
                self.focused_window = Some(window.id);
                println!("ld after add: {self:?}");
                Ok(())
            }
            Err(e) => Err(e),
        }
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
        self.root.resize_window(
            window_id,
            direction,
            RESIZE_AMOUNT,
            self.config.window_padding(),
        )
    }
}
