mod axis;
pub(crate) mod leaf;
pub(crate) mod split;

pub use crate::container::axis::Axis;
use crate::container::leaf::Leaf;
use crate::container::split::Split;
use crate::error::{Error, Result};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};

pub enum RemoveResult {
    BecomeEmpty,
    Removed,
    NotFound,
}

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

#[cfg(test)]
impl Window {
    pub fn dummy(id: WindowId) -> Self {
        Window {
            id,
            min_width: 100.0,
            min_height: 100.0,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(super) enum Container {
    Leaf(Leaf),
    Split(Split),
}

impl Container {
    pub fn min_width(&self) -> f64 {
        match self {
            Self::Leaf(leaf) => leaf.window.min_width,
            Self::Split(split) => split.min_width(),
        }
    }

    pub fn min_height(&self) -> f64 {
        match self {
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
            Self::Leaf(_) => Err(Error::CannotAddWindowToLeaf),
            Self::Split(split) => split.add_window(window, padding),
        }
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        match self {
            Self::Split(split) => split.split(axis),
            Self::Leaf(leaf) => {
                *self = Self::Split(leaf.split(axis));
                Ok(())
            }
        }
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        match self {
            Self::Leaf(leaf) => HashSet::from([leaf.window.id]),
            Self::Split(split) => split.window_ids(),
        }
    }

    pub fn window_bounds_by_id(&self) -> HashMap<WindowId, Bounds> {
        match self {
            Self::Leaf(leaf) => HashMap::from([(leaf.window.id, leaf.bounds)]),
            Self::Split(split) => split.window_bounds_by_id(),
        }
    }

    pub(super) fn remove_window(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<RemoveResult> {
        match self {
            Self::Leaf(leaf) if leaf.window.id == window_id => Ok(RemoveResult::BecomeEmpty),
            Self::Split(split) => split.remove_window(window_id, padding),
            Self::Leaf(_) => Ok(RemoveResult::NotFound),
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
            Self::Leaf(_) => Err(Error::WindowNotFound),
            Self::Split(split) => split.resize_window(window_id, direction, padding),
        }
    }

    fn contains_window(&self, search: WindowId) -> bool {
        self.find_window(search).is_some()
    }

    /// Get the bounds of a `Container`.
    pub fn bounds(&self) -> Bounds {
        match self {
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
            Self::Leaf { .. } => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use Axis::{Horizontal, Vertical};
    use core_graphics::Bounds;

    const PADDING_VALUES: &[f64] = &[0.0, 5.0, 10.0, 17.5];
    const AXES: &[Axis] = &[Horizontal, Vertical];
    const CHILD_NUMS: &[usize] = &[1, 2, 3, 4, 5, 6, 7, 8];

    const EPSILON: f64 = 1e-10;

    const fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    pub fn dummy_bounds() -> Bounds {
        Bounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    // Get the `(position, size)` of the give bounds along that given axis.
    fn along(b: &Bounds, axis: Axis) -> (f64, f64) {
        match axis {
            Vertical => (b.y, b.height),
            Horizontal => (b.x, b.width),
        }
    }

    // Get the `(position, size)` of the give bounds along the axis
    // perpendicular to `axis`.
    fn perpendicular(b: &Bounds, axis: Axis) -> (f64, f64) {
        let perp_axis = match axis {
            Vertical => Horizontal,
            Horizontal => Vertical,
        };
        along(b, perp_axis)
    }

    fn transpose(b: Bounds) -> Bounds {
        Bounds {
            x: b.y,
            y: b.x,
            width: b.height,
            height: b.width,
        }
    }

    #[test]
    fn spread_bounds() {
        let original = dummy_bounds();

        for &padding in PADDING_VALUES {
            for &axis in AXES {
                for &n in CHILD_NUMS {
                    let out = spread_bounds_along_axis(original, axis, n, padding);
                    assert_eq!(out.len(), n);
                    assert!(approx(out[0].x, original.x + padding));
                    assert!(approx(out[0].y, original.y + padding));

                    let total_inner_gap = (n - 1) as f64 * padding;

                    let available_space = match axis {
                        Vertical => original.height - 2.0 * padding - total_inner_gap,
                        Horizontal => original.width - 2.0 * padding - total_inner_gap,
                    };
                    let child_share = available_space / n as f64;
                    let expected_unchanged_dimension = match axis {
                        Vertical => original.width - 2.0 * padding,
                        Horizontal => original.height - 2.0 * padding,
                    };

                    let last = &out[n - 1];
                    match axis {
                        Vertical => assert!(approx(
                            last.y + last.height,
                            original.y + original.height - padding
                        )),
                        Horizontal => assert!(approx(
                            last.x + last.width,
                            original.x + original.width - padding
                        )),
                    }

                    // The original bounds (x/y) positions dependent on axis
                    let (orig_pos, _) = along(&original, axis);
                    let (original_perp_pos, _) = perpendicular(&original, axis);

                    for (i, b) in out.iter().enumerate() {
                        let (pos, size) = along(b, axis);
                        let (perp_pos, perp_size) = perpendicular(b, axis);

                        // Each child should be the same, correct size
                        assert!(approx(size, child_share));
                        // The dimension perpendicular to the axis we are
                        // spreading along should just be what it was originally
                        // (accounting for padding)
                        assert!(approx(perp_size, expected_unchanged_dimension));
                        // Accounting for padding, all children should have the
                        // same (x/y) value perpendicular to the spreading axis
                        assert!(approx(perp_pos, original_perp_pos + padding));
                        // All children should have equally spaced including
                        // padding in between them
                        assert!(approx(
                            pos,
                            orig_pos + padding + i as f64 * (child_share + padding)
                        ));
                    }

                    let covered = match axis {
                        Vertical => out.iter().map(|b| b.height).sum(),
                        Horizontal => out.iter().map(|b| b.width).sum(),
                    };
                    assert!(approx(covered, available_space));
                }
            }
        }
    }

    #[test]
    fn spread_bounds_along_axis_symmetry() {
        let original = dummy_bounds();
        let transposed = transpose(original);

        let padding = 12.0;
        let n = 4;

        let h = spread_bounds_along_axis(original, Horizontal, n, padding);
        let v = spread_bounds_along_axis(transposed, Vertical, n, padding);

        for (bh, bv) in h.iter().zip(v.iter()) {
            assert!(approx(bh.width, bv.height));
            assert!(approx(bh.height, bv.width));
        }
    }
}
