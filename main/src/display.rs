use crate::{Error::AxUi, Result, container::Container, window::Window};
use core_graphics::{Bounds, WindowId};
use std::collections::HashSet;

#[derive(Debug)]
pub(crate) struct Display {
    bounds: Bounds,
    root: Container,
    focused_window: Option<WindowId>,
}

impl Display {
    pub(crate) fn new(cg_bounds: Bounds) -> Self {
        // Core Graphics bounds do not include the Apple status bar, we need to
        // subtract the height of said bar to stop vertically split windows from
        // overlapping each other.
        const STATUS_BAR_HEIGHT: f64 = 37.0;

        let bounds = Bounds {
            height: cg_bounds.height - STATUS_BAR_HEIGHT,
            y: cg_bounds.y + STATUS_BAR_HEIGHT,
            ..cg_bounds
        };

        Display {
            bounds,
            root: Container::Empty { bounds },
            focused_window: None,
        }
    }

    pub(crate) fn window_ids(&self) -> HashSet<WindowId> {
        self.root.window_ids()
    }

    pub(crate) fn cg_windows(&self) -> HashSet<&core_graphics::Window> {
        self.root.cg_windows()
    }

    pub fn find_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        self.root.find_window_mut(window_id)
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<bool> {
        self.root.remove_window(window_id)
    }

    pub fn add_window(&mut self, window: core_graphics::Window) -> Result<()> {
        if let Some(focused_id) = self.focused_window {
            if let Some(container) = self.root.get_parent_of_window_mut(focused_id) {
                return container.add_window(window);
            }
        }

        self.root.add_window(window)
    }

    pub fn focus_window(&self, window_id: WindowId) -> Result<()> {
        if let Some(window) = self.root.find_window(window_id) {
            window.ax().focus().map_err(AxUi)
        } else {
            Err(crate::Error::WindowNotFound)
        }
    }

    pub fn get_parent_of_window(&mut self, window_id: WindowId) -> Option<&mut Container> {
        self.root.get_parent_of_window_mut(window_id)
    }

    pub fn get_leaf_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Container> {
        self.root.get_leaf_of_window_mut(window_id)
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) {
        self.focused_window = Some(window_id);
    }
}
