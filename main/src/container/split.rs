use crate::container::leaf::Leaf;
use crate::container::{Axis, Container, spread_bounds_in_direction};
use crate::error::{Error, Result};
use crate::window::Window;
use core_graphics::{Bounds, WindowId};
use std::collections::HashSet;

#[derive(Debug)]
pub(super) struct Split {
    bounds: Bounds,
    axis: Axis,
    children: Vec<Container>,
}

impl Split {
    pub(super) fn from_single_window(bounds: Bounds, axis: Axis, window: Window) -> Self {
        Self {
            bounds,
            axis,
            children: vec![Container::Leaf(Leaf::new(bounds, window))],
        }
    }

    // To add a window to a split container:
    //  1. Create the new window and add it to the split's children.
    //  2. Spread the containers bounds across the now N children.
    //  3. Resize all children using those new bounds.
    pub(super) fn add_window(
        &mut self,
        cg_window: core_graphics::Window,
        padding: f64,
    ) -> Result<()> {
        let num_new_children = self.children.len() + 1;
        let new_children_bounds =
            spread_bounds_in_direction(self.bounds, self.axis, num_new_children, padding);

        let window_bounds = new_children_bounds[num_new_children - 1];
        let mut new_window = Window::try_new(cg_window, window_bounds)?;
        new_window.init()?;

        self.children
            .push(Container::Leaf(Leaf::new(window_bounds, new_window)));

        for (child, new_bounds) in self.children.iter_mut().zip(new_children_bounds) {
            child.resize(new_bounds, padding)?;
        }

        Ok(())
    }

    pub(super) fn split(&self, axis: Axis) -> Result<Container> {
        if self.children.len() < 2 {
            Ok(Container::Split(Self {
                bounds: self.bounds,
                axis,
                children: self.children.clone(),
            }))
        } else {
            Err(Error::CannotSplitAlreadySplitContainer)
        }
    }

    pub(super) fn all_windows(&self) -> Vec<&Window> {
        let mut windows = Vec::new();
        for child in &self.children {
            windows.extend(child.all_windows());
        }

        windows
    }

    pub(super) fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        self.children
            .iter()
            .find_map(|child| child.find_window(window_id))
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        self.children
            .iter()
            .flat_map(|child| child.window_ids())
            .collect()
    }

    pub(super) fn windows_mut(&mut self) -> HashSet<&mut Window> {
        self.children
            .iter_mut()
            .flat_map(|child| child.windows_mut())
            .collect()
    }

    pub(super) fn remove_window(&mut self, window_id: WindowId) -> Result<Option<Window>> {
        if let Some(i) = self
            .children
            .iter()
            .position(|child| child.is_parent_leaf(window_id))
        {
            let removed_child = self.children.remove(i);
            self.children.retain(|c| !c.is_empty());

            if self.children.is_empty() {
                *self = Self::Empty { bounds: *bounds };
            } else {
                let new_children_bounds =
                    spread_bounds_in_direction(*bounds, *axis, children.len(), padding);
                for (child, new_bounds) in children.iter_mut().zip(new_children_bounds) {
                    child.resize(new_bounds, padding)?;
                }
            }
            return Ok(Some(match removed_child {
                Self::Leaf { window, .. } => window,
                _ => unreachable!(),
            }));
        }

        for child in children.iter_mut() {
            if let Some(window) = child.remove_window(window_id, padding)? {
                children.retain(|c| !c.is_empty());
                return Ok(Some(window));
            }
        }

        Ok(None)
    }
}
