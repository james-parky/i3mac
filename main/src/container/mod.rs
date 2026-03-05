mod axis;
mod tests;

pub use crate::container::axis::Axis;
use crate::error::Error::CannotAddWindowToLeaf;
use crate::error::{Error, Result};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};

const RESIZE_AMOUNT: f64 = 50.0;

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
    Empty {
        bounds: Bounds,
    },
    Leaf {
        bounds: Bounds,
        padding: f64,
        window: Window,
    },
    Split {
        bounds: Bounds,
        axis: Axis,
        padding: f64,
        children: Vec<Container>,
    },
}

impl Container {
    pub const fn window_bounds(&self) -> Option<Bounds> {
        match self {
            Container::Leaf { bounds, .. } => Some(*bounds),
            _ => None,
        }
    }

    fn add_window_to_empty(&mut self, window: Window, padding: f64) -> Result<()> {
        let Self::Empty { bounds } = self else {
            return Err(CannotAddWindowToLeaf);
        };

        let leaf_bounds = spread_bounds_along_axis(*bounds, Axis::default(), 1, padding);
        *self = Self::Split {
            bounds: *bounds,
            axis: Axis::default(),
            padding,
            children: vec![Self::Leaf {
                bounds: leaf_bounds[0],
                padding,
                window,
            }],
        };

        Ok(())
    }

    pub fn min_width(&self) -> f64 {
        match self {
            Self::Leaf { window, .. } => window.min_width,
            Self::Split { children, .. } => {
                children.iter().map(|c| c.min_width()).fold(0.0, f64::max)
            }
            Self::Empty { .. } => 0.0,
        }
    }

    pub fn min_height(&self) -> f64 {
        match self {
            Self::Leaf { window, .. } => window.min_height,
            Self::Split { children, .. } => {
                children.iter().map(|c| c.min_height()).fold(0.0, f64::max)
            }
            Self::Empty { .. } => 0.0,
        }
    }

    // To add a window to a split container:
    //  1. Create the new window and add it to the split's children.
    //  2. Spread the containers bounds across the now N children.
    //  3. Resize all children using those new bounds.
    fn add_window_to_split(&mut self, window: Window, padding: f64) -> Result<()> {
        let Self::Split {
            bounds,
            axis,
            children,
            ..
        } = self
        else {
            return Err(Error::ExpectedSplitContainer);
        };

        let num_new_children = children.len() + 1;
        let new_bounds = spread_bounds_along_axis(*bounds, *axis, num_new_children, padding);

        if children
            .iter()
            .zip(&new_bounds)
            .any(|(c, b)| b.width < c.min_width() || b.height < c.min_height())
        {
            return Err(Error::CannotFitWindow);
        }

        // Also check new window
        let last_bounds = new_bounds.last().unwrap();
        if last_bounds.width < window.min_width || last_bounds.height < window.min_height {
            return Err(Error::CannotFitWindow);
        }

        children.push(Container::Leaf {
            bounds: new_bounds[num_new_children - 1],
            padding,
            window,
        });

        for (child, new_bounds) in children.iter_mut().zip(new_bounds) {
            child.resize(new_bounds)?;
        }

        Ok(())
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
            Self::Empty { .. } => self.add_window_to_empty(window, padding),
            Self::Leaf { .. } => Err(Error::CannotAddWindowToLeaf),
            Self::Split { .. } => self.add_window_to_split(window, padding),
        }
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        match self {
            Self::Empty { .. } => Err(Error::CannotSplitEmptyContainer),
            Self::Split {
                axis: old,
                children,
                ..
            } if children.len() < 2 => {
                *old = axis;
                Ok(())
            }
            Self::Split { .. } => Err(Error::CannotSplitAlreadySplitContainer),
            Self::Leaf {
                bounds,
                padding,
                window,
            } => {
                let saved_bounds = *bounds;
                let saved_padding = *padding;
                let saved_window = *window;
                let outer_bounds = saved_bounds.with_pad(-saved_padding);

                *self = Container::Split {
                    bounds: outer_bounds,
                    axis,
                    padding: saved_padding,
                    children: vec![Container::Leaf {
                        bounds: saved_bounds,
                        padding: saved_padding,
                        window: saved_window,
                    }],
                };

                Ok(())
            }
        }
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        match self {
            Self::Empty { .. } => HashSet::new(),
            Self::Leaf { window, .. } => HashSet::from([window.id]),
            Self::Split { children, .. } => children
                .iter()
                .flat_map(|child| child.window_ids())
                .collect(),
        }
    }

    pub fn window_bounds_by_id(&self) -> HashMap<WindowId, Bounds> {
        match self {
            Self::Empty { .. } => HashMap::new(),
            Self::Leaf { window, bounds, .. } => HashMap::from([(window.id, *bounds)]),
            Self::Split { children, .. } => children
                .iter()
                .flat_map(|child| child.window_bounds_by_id())
                .collect(),
        }
    }

    fn remove_window_from_leaf(&mut self, target: WindowId) -> Result<Option<WindowId>> {
        match self {
            Self::Leaf { window, .. } if window.id == target => {
                let old = std::mem::replace(
                    self,
                    Self::Empty {
                        bounds: self.bounds(),
                    },
                );
                if let Container::Leaf { window, .. } = old {
                    Ok(Some(window.id))
                } else {
                    unreachable!()
                }
            }
            _ => Ok(None),
        }
    }

    fn is_parent_leaf(&self, target: WindowId) -> bool {
        matches!(self, Self::Leaf{ window,.. } if window.id == target)
    }

    fn remove_window_from_split(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<Option<WindowId>> {
        if let Container::Split {
            children,
            bounds,
            axis,
            ..
        } = self
        {
            if let Some(pos) = children
                .iter()
                .position(|c| matches!(c, Container::Leaf { window, .. } if window.id == window_id))
            {
                let removed = children.remove(pos);
                children.retain(|c| !matches!(c, Container::Empty { .. }));
                if children.is_empty() {
                    *self = Container::Empty { bounds: *bounds };
                } else {
                    let new_bounds =
                        spread_bounds_along_axis(*bounds, *axis, children.len(), padding);
                    for (child, b) in children.iter_mut().zip(new_bounds) {
                        child.resize(b)?;
                    }
                }

                if let Container::Leaf { window, .. } = removed {
                    return Ok(Some(window.id));
                } else {
                    unreachable!()
                }
            }

            let mut found_id: Option<WindowId> = None;
            for child in children.iter_mut() {
                if let Some(id) = child.remove_window(window_id, padding)? {
                    found_id = Some(id);
                    break;
                }
            }

            if let Some(id) = found_id {
                // Drop now empty kids
                let m = children.len();
                children.retain(|c| !matches!(c, Container::Empty { .. }));

                let saved_bounds = *bounds;
                let saved_axis = *axis;
                let n = children.len();

                if n == 0 {
                    *self = Container::Empty {
                        bounds: saved_bounds,
                    };
                } else if n < m {
                    let new_bounds = spread_bounds_along_axis(saved_bounds, saved_axis, n, padding);
                    if let Container::Split { children, .. } = self {
                        for (child, b) in children.iter_mut().zip(new_bounds) {
                            child.resize(b)?;
                        }
                    }
                }

                return Ok(Some(id));
            }

            Ok(None)
        } else {
            Ok(None)
        }
    }

    pub(super) fn remove_window(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<Option<WindowId>> {
        match self {
            Self::Empty { .. } => Ok(None),
            Self::Leaf { .. } => self.remove_window_from_leaf(window_id),
            Self::Split { .. } => self.remove_window_from_split(window_id, padding),
        }
    }

    pub fn parent_leaf_of_window_mut(&mut self, target: WindowId) -> Option<&mut Self> {
        match self {
            Self::Leaf { window, .. } if window.id == target => Some(self),
            Self::Split { children, .. } => {
                for child in children {
                    if let Some(parent) = child.parent_leaf_of_window_mut(target) {
                        return Some(parent);
                    }
                }
                None
            }
            _ => None,
        }
    }

    // The parent of a window is defined as:
    //  - The immediate split ancestor of the window -- in that, all windows are
    //    children of a leaf, and all leaves of direct children of some split
    //    container. Return that split container.
    pub fn get_parent_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Self> {
        if !matches!(self, Self::Split { .. }) {
            return None;
        }

        let is_direct_split_ancestor = match self {
            Self::Split { children, .. } => {
                children.iter().any(|child| child.is_parent_leaf(window_id))
            }
            _ => false,
        };

        if is_direct_split_ancestor {
            return Some(self);
        }

        if let Self::Split { children, .. } = self {
            for child in children.iter_mut() {
                if let Some(parent) = child.get_parent_of_window_mut(window_id) {
                    return Some(parent);
                }
            }
        }

        None
    }

    // To resize a container, resize its own bounds, then resize all its
    // children recursively.
    fn resize(&mut self, new_bounds: Bounds) -> Result<()> {
        match self {
            Self::Empty { bounds, .. } => *bounds = new_bounds,
            Self::Leaf { bounds, .. } => *bounds = new_bounds,
            Self::Split {
                bounds,
                children,
                axis,
                padding: p,
            } => {
                let old_bounds = *bounds;
                *bounds = new_bounds;

                for child in children.iter_mut() {
                    let cb = child.bounds();
                    let new_cb = match axis {
                        Axis::Horizontal => {
                            let x_ratio = (cb.x - old_bounds.x) / old_bounds.width;
                            let w_ratio = cb.width / old_bounds.width;
                            Bounds {
                                x: new_bounds.x + x_ratio * new_bounds.width,
                                y: bounds.y + *p,
                                width: w_ratio * new_bounds.width,
                                height: bounds.height - 2.0 * *p,
                            }
                        }
                        Axis::Vertical => {
                            let y_ratio = (cb.y - old_bounds.y) / old_bounds.height;
                            let h_ratio = cb.height / old_bounds.height;
                            Bounds {
                                x: bounds.x + *p,
                                y: new_bounds.y + y_ratio * new_bounds.height,
                                width: bounds.width - 2.0 * *p,
                                height: h_ratio * new_bounds.height,
                            }
                        }
                    };

                    child.resize(new_cb)?;
                }
            }
        }

        Ok(())
    }

    pub fn resize_window(
        &mut self,
        window_id: WindowId,
        direction: Direction,
        padding: f64,
    ) -> Result<()> {
        let Self::Split { children, axis, .. } = self else {
            return Err(Error::WindowNotFound);
        };

        let i = children
            .iter()
            .position(|c| c.contains_window(window_id))
            .ok_or(Error::WindowNotFound)?;

        if !axis.can_resize_in_direction(direction) {
            return children[i].resize_window(window_id, direction, padding);
        }

        let n = if i == 0 { 1 } else { i - 1 };
        let (left, right) = children.split_at_mut(i.max(n));
        let (a, b) = (&mut left[i.min(n)], &mut right[0]);

        Self::resize_at_split(*axis, a, b, direction, padding)
    }

    fn resize_at_split(
        axis: Axis,
        a: &mut Self,
        b: &mut Self,
        direction: Direction,
        padding: f64,
    ) -> Result<()> {
        use Axis::*;
        use Direction::*;

        let (first_bounds, second_bounds) = (a.bounds(), b.bounds());

        let delta = match direction {
            Left | Up => -RESIZE_AMOUNT,
            Right | Down => RESIZE_AMOUNT,
        };

        // Arbitrary reasonable constant that stop windows getting too
        // small. When this value is too small, the OS doesn't let the
        // smaller window get smaller, but this code will make the larger
        // window get larger and thus they overlap.
        const MIN_SIZE: f64 = 200.0;

        let midpoint = match axis {
            Vertical => first_bounds.y + first_bounds.height + padding / 2.0,
            Horizontal => first_bounds.x + first_bounds.width + padding / 2.0,
        };

        let new_midpoint = match axis {
            Vertical => (midpoint + delta)
                .max(first_bounds.y + MIN_SIZE)
                .min(first_bounds.y + first_bounds.height + second_bounds.height - MIN_SIZE),
            Horizontal => (midpoint + delta)
                .max(first_bounds.x + MIN_SIZE)
                .min(first_bounds.x + first_bounds.width + second_bounds.width - MIN_SIZE),
        };

        // The new midpoint is already less than a pixel away from the
        // current midpoint, so it cannot move.
        if (new_midpoint - midpoint).abs() < 1.0 {
            return Ok(());
        }

        let new_first_bounds = match axis {
            Vertical => Bounds {
                height: new_midpoint - first_bounds.y - padding / 2.0,
                ..first_bounds
            },
            Horizontal => Bounds {
                width: new_midpoint - first_bounds.x - padding / 2.0,
                ..first_bounds
            },
        };

        let new_second_bounds = match axis {
            Vertical => Bounds {
                y: new_midpoint + padding / 2.0,
                height: second_bounds.y + second_bounds.height - new_midpoint - padding / 2.0,
                ..second_bounds
            },
            Horizontal => Bounds {
                x: new_midpoint + padding / 2.0,
                width: second_bounds.x + second_bounds.width - new_midpoint - padding / 2.0,
                ..second_bounds
            },
        };

        a.resize(new_first_bounds)?;
        b.resize(new_second_bounds)?;

        Ok(())
    }

    fn contains_window(&self, search: WindowId) -> bool {
        self.find_window(search).is_some()
    }

    /// Get the bounds of a `Container`.
    fn bounds(&self) -> Bounds {
        match self {
            Self::Empty { bounds } => *bounds,
            Self::Leaf { bounds, .. } => *bounds,
            Self::Split { bounds, .. } => *bounds,
        }
    }

    /// Return `Some(target)` if the container itself, or one of its managed
    /// children manages the `target` window; otherwise return `None`.
    pub(super) fn find_window(&self, target: WindowId) -> Option<WindowId> {
        match self {
            Self::Leaf { window, .. } if window.id == target => Some(window.id),
            Self::Split { children, .. } => {
                children.iter().find_map(|child| child.find_window(target))
            }
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
