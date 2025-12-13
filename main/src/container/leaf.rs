use crate::container::split::Split;
use crate::container::{Axis, Container};
use crate::error::{Error, Result};
use crate::window::Window;
use core_graphics::{Bounds, WindowId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(super) struct Leaf {
    bounds: Bounds,
    window: Window,
}

impl Leaf {
    pub(super) fn new(bounds: Bounds, window: Window) -> Self {
        Self { bounds, window }
    }

    pub(super) fn add_window(&mut self, _: core_graphics::Window, _: f64) -> Result<Container> {
        Err(Error::CannotAddWindowToLeaf)
    }

    pub(super) fn split(&self, axis: Axis) -> Result<Container> {
        Ok(Container::Split(Split::from_single_window(
            self.bounds,
            axis,
            self.window.clone(),
        )))
    }

    pub(super) fn all_windows(&self) -> Vec<&Window> {
        vec![&self.window]
    }

    pub(super) fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        if self.window.cg().number() == window_id {
            Some(&self.window)
        } else {
            None
        }
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        HashSet::from([self.window.cg().number()])
    }

    pub(super) fn windows_mut(&mut self) -> HashSet<&mut Window> {
        HashSet::from([&mut self.window])
    }

    pub(super) fn remove_window(&mut self, window_id: WindowId) -> Result<Option<Window>> {
        if self.window.cg().number() != window_id {
            return Ok(None);
        }

        Ok(Some(self.window.clone()))
    }
}
