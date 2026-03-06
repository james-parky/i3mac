mod axis;
mod leaf;
mod split;
mod tests;

pub use crate::container::axis::Axis;
use crate::container::leaf::Leaf;
use crate::container::split::{RemoveResult, Split};
use crate::error::{Error, Result};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Window {
    pub id: WindowId,
    pub min_width: f64,
    pub min_height: f64,
}

impl From<crate::window::Window> for Window {
    fn from(window: crate::window::Window) -> Self {
        let min_size = window.ax().min_size().unwrap_or_default();
        Self {
            id: window.cg().number(),
            min_height: min_size.height,
            min_width: min_size.width,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(super) enum Container {
    Empty(Empty),
    Leaf(Leaf),
    Split(Split),
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(super) struct Empty {
    bounds: Bounds,
}

impl Empty {
    pub fn new(bounds: Bounds) -> Self {
        Self { bounds }
    }

    fn add_window(&self, window: Window, padding: f64) -> Split {
        let leaf_bounds = spread_bounds_along_axis(self.bounds, Axis::default(), 1, padding);
        let children = vec![Container::Leaf(Leaf::new(leaf_bounds[0], padding, window))];

        Split::new(self.bounds, Axis::default(), padding, children)
    }
}

impl Container {
    pub fn min_width(&self) -> f64 {
        match self {
            Self::Empty { .. } => 0.0,
            Self::Leaf(leaf) => leaf.window.min_width,
            Self::Split(split) => split.min_width(),
        }
    }

    pub fn min_height(&self) -> f64 {
        match self {
            Self::Empty { .. } => 0.0,
            Self::Leaf(leaf) => leaf.window.min_height,
            Self::Split(split) => split.min_height(),
        }
    }

    // Rules for adding a window to a container:
    //  - If a container is empty, create a default split and add the new window
    //    as a single child.
    //  - If a container is a leaf, return an error.
    //  - If a container is a split, add a new child, and adjust the bounds of
    //    the existing children. This requires adjusting the bounds of all
    //    existing children in said split, recursively.
    pub fn add_window(&mut self, window: Window, padding: f64) -> Result<()> {
        match self {
            Self::Empty(empty) => {
                *self = Self::Split(empty.add_window(window, padding));
                Ok(())
            }
            Self::Leaf(_) => Err(Error::CannotAddWindowToLeaf),
            Self::Split(split) => split.add_window(window, padding),
        }
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        match self {
            Self::Empty(_) => Err(Error::CannotSplitEmptyContainer),
            Self::Split(split) => split.split(axis),
            Self::Leaf(leaf) => {
                *self = Self::Split(leaf.split(axis));
                Ok(())
            }
        }
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        match self {
            Self::Empty(_) => HashSet::new(),
            Self::Leaf(leaf) => HashSet::from([leaf.window.id]),
            Self::Split(split) => split.window_ids(),
        }
    }

    pub fn window_bounds_by_id(&self) -> HashMap<WindowId, Bounds> {
        match self {
            Self::Empty(_) => HashMap::new(),
            Self::Leaf(leaf) => HashMap::from([(leaf.window.id, leaf.bounds)]),
            Self::Split(split) => split.window_bounds_by_id(),
        }
    }

    pub(super) fn remove_window(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<Option<WindowId>> {
        match self {
            Self::Leaf(leaf) if leaf.window.id == window_id => {
                *self = Self::Empty(Empty::new(leaf.bounds));
                Ok(Some(window_id))
            }
            Self::Split(split) => match split.remove_window(window_id, padding)? {
                RemoveResult::BecomeEmpty => {
                    *self = Self::Empty(Empty::new(self.bounds()));
                    Ok(Some(window_id))
                }
                RemoveResult::Removed => Ok(Some(window_id)),
                RemoveResult::DidntRemove => Ok(None),
            },
            Self::Empty(_) | Self::Leaf(_) => Ok(None),
        }
    }

    pub fn parent_leaf_of_window_mut(&mut self, target: WindowId) -> Option<&mut Self> {
        match self {
            Self::Leaf(leaf) if leaf.window.id == target => Some(self),
            Self::Split(split) => split
                .children
                .iter_mut()
                .find_map(|c| c.parent_leaf_of_window_mut(target)),
            _ => None,
        }
    }

    // The parent of a window is defined as:
    //  - The immediate split ancestor of the window -- in that, all windows are
    //    children of a leaf, and all leaves of direct children of some split
    //    container. Return that split container.
    pub fn get_parent_of_window_mut(&mut self, target: WindowId) -> Option<&mut Self> {
        let Self::Split(split) = self else {
            return None;
        };

        let is_direct_split_ancestor = split
            .children
            .iter()
            .any(|child| matches!(child, Container::Leaf(leaf) if leaf.window.id == target));

        if is_direct_split_ancestor {
            return Some(self);
        }

        // Ridiculous borrow check appeasement
        let Self::Split(split) = self else {
            return None;
        };

        split
            .children
            .iter_mut()
            .find_map(|c| c.get_parent_of_window_mut(target))
    }

    // To resize a container, resize its own bounds, then resize all its
    // children recursively.
    fn resize(&mut self, new_bounds: Bounds) -> Result<()> {
        match self {
            Self::Empty(empty) => empty.bounds = new_bounds,
            Self::Leaf(leaf) => leaf.bounds = new_bounds,
            Self::Split(split) => split.resize(new_bounds)?,
        }

        Ok(())
    }

    pub fn resize_window(
        &mut self,
        window_id: WindowId,
        direction: Direction,
        padding: f64,
    ) -> Result<()> {
        match self {
            // TODO: error
            Self::Empty(_) | Self::Leaf(_) => Err(Error::WindowNotFound),
            Self::Split(split) => split.resize_window(window_id, direction, padding),
        }
    }

    fn contains_window(&self, search: WindowId) -> bool {
        self.find_window(search).is_some()
    }

    /// Get the bounds of a `Container`.
    fn bounds(&self) -> Bounds {
        match self {
            Self::Empty(empty) => empty.bounds,
            Self::Leaf(leaf) => leaf.bounds,
            Self::Split(split) => split.bounds,
        }
    }

    /// Return `Some(target)` if the container itself, or one of its managed
    /// children manages the `target` window; otherwise return `None`.
    pub(super) fn find_window(&self, target: WindowId) -> Option<WindowId> {
        match self {
            Self::Leaf(leaf) if leaf.window.id == target => Some(leaf.window.id),
            Self::Split(split) => split
                .children
                .iter()
                .find_map(|child| child.find_window(target)),
            Self::Empty { .. } | Self::Leaf { .. } => None,
        }
    }
}

/// Return a list of `n` bounds spread equally across the provided region,
/// accounting for edge, and inter-element padding.
fn spread_bounds_along_axis(original: Bounds, axis: Axis, n: usize, padding: f64) -> Vec<Bounds> {
    use Axis::*;

    assert_ne!(n, 0);

    let total_inner_gap = (n - 1) as f64 * (padding); // half padding between children
    let length_to_split = match axis {
        Horizontal => original.width,
        Vertical => original.height,
    };
    let available_space = length_to_split - 2.0 * padding - total_inner_gap;
    let each_child_length = available_space / n as f64;

    (0..n)
        .map(|i| match axis {
            Horizontal => Bounds {
                x: original.x + padding + i as f64 * (each_child_length + padding),
                y: original.y + padding,
                width: each_child_length,
                height: original.height - 2.0 * padding,
            },
            Vertical => Bounds {
                x: original.x + padding,
                y: original.y + padding + i as f64 * (each_child_length + padding),
                width: original.width - 2.0 * padding,
                height: each_child_length,
            },
        })
        .collect()
}
