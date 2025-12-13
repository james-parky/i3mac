use crate::container::split::Split;
use crate::container::{Axis, Container};
use crate::error::Result;
use crate::window::Window;
use core_graphics::{Bounds, WindowId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(super) struct Empty {
    bounds: Bounds,
}

impl Empty {
    pub(super) fn add_window(
        &mut self,
        cg_window: core_graphics::Window,
        padding: f64,
    ) -> Result<Container> {
        let window_bounds = self.bounds.with_pad(padding);
        let mut window = Window::try_new(cg_window, window_bounds)?;
        window.init()?;

        Ok(Container::Split(Split::from_single_window(
            window_bounds,
            Axis::default(),
            window,
        )))
    }

    pub(super) fn all_windows(&self) -> Vec<&Window> {
        Vec::new()
    }

    pub(super) fn find_window(&self, _: WindowId) -> Option<&Window> {
        None
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        HashSet::new()
    }

    pub(super) fn windows_mut(&mut self) -> HashSet<&mut Window> {
        HashSet::new()
    }

    pub(super) fn remove_window(&mut self, _: WindowId, _: f64) -> Result<Option<Window>> {
        Ok(None)
    }
}
