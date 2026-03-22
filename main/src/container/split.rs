use crate::container::RemoveResult;
use crate::{
    container::{Axis, Container, Window, leaf::Leaf, spread_bounds_along_axis},
    error::{Error, Result},
    window_manager,
};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct Split {
    pub bounds: Bounds,
    pub axis: Axis,
    pub padding: f64,
    pub children: Vec<Container>,
}

impl Split {
    pub fn new(bounds: Bounds, axis: Axis, padding: f64, children: Vec<Container>) -> Self {
        Self {
            bounds,
            axis,
            padding,
            children,
        }
    }

    pub fn min_width(&self) -> f64 {
        self.children
            .iter()
            .map(|c| c.min_width())
            .fold(0.0, f64::max)
    }

    pub fn min_height(&self) -> f64 {
        self.children
            .iter()
            .map(|c| c.min_height())
            .fold(0.0, f64::max)
    }

    // To add a window to a split container:
    //  1. Create the new window and add it to the split's children.
    //  2. Spread the containers bounds across the now N children.
    //  3. Resize all children using those new bounds.
    pub fn add_window(&mut self, window: Window, padding: f64) -> Result<()> {
        let num_new_children = self.children.len() + 1;
        let new_bounds =
            spread_bounds_along_axis(self.bounds, self.axis, num_new_children, padding);

        if self
            .children
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

        let new_child = Container::Leaf(Leaf::new(*new_bounds.last().unwrap(), padding, window));
        self.children.push(new_child);

        for (child, new_bounds) in self.children.iter_mut().zip(new_bounds) {
            child.resize(new_bounds)?;
        }

        Ok(())
    }

    pub fn window_ids(&self) -> HashSet<WindowId> {
        self.children
            .iter()
            .flat_map(|child| child.window_ids())
            .collect()
    }

    pub fn window_bounds_by_id(&self) -> HashMap<WindowId, Bounds> {
        self.children
            .iter()
            .flat_map(|child| child.window_bounds_by_id())
            .collect()
    }

    pub fn resize(&mut self, new_bounds: Bounds) -> Result<()> {
        let old_bounds = self.bounds;
        self.bounds = new_bounds;

        for child in self.children.iter_mut() {
            let cb = child.bounds();
            let new_cb = match self.axis {
                Axis::Horizontal => {
                    let x_ratio = (cb.x - old_bounds.x) / old_bounds.width;
                    let w_ratio = cb.width / old_bounds.width;
                    Bounds {
                        x: new_bounds.x + x_ratio * new_bounds.width,
                        y: self.bounds.y + self.padding,
                        width: w_ratio * new_bounds.width,
                        height: self.bounds.height - 2.0 * self.padding,
                    }
                }
                Axis::Vertical => {
                    let y_ratio = (cb.y - old_bounds.y) / old_bounds.height;
                    let h_ratio = cb.height / old_bounds.height;
                    Bounds {
                        x: self.bounds.x + self.padding,
                        y: new_bounds.y + y_ratio * new_bounds.height,
                        width: self.bounds.width - 2.0 * self.padding,
                        height: h_ratio * new_bounds.height,
                    }
                }
            };

            child.resize(new_cb)?;
        }

        Ok(())
    }

    pub fn remove_window(&mut self, id: WindowId, padding: f64) -> Result<RemoveResult> {
        if let Some(pos) = self
            .children
            .iter()
            .position(|c| matches!(c, Container::Leaf(leaf) if leaf.window.id == id))
        {
            self.children.remove(pos);

            if self.children.is_empty() {
                return Ok(RemoveResult::BecomeEmpty);
            }

            let new_bounds =
                spread_bounds_along_axis(self.bounds, self.axis, self.children.len(), padding);
            for (child, b) in self.children.iter_mut().zip(new_bounds) {
                child.resize(b)?;
            }

            return Ok(RemoveResult::Removed);
        }

        // Recursive case
        for i in 0..self.children.len() {
            match self.children[i].remove_window(id, padding)? {
                RemoveResult::NotFound => continue,
                RemoveResult::Removed => {
                    return Ok(RemoveResult::Removed);
                }
                RemoveResult::BecomeEmpty => {
                    self.children.remove(i);

                    if self.children.is_empty() {
                        return Ok(RemoveResult::BecomeEmpty);
                    }

                    let new_bounds = spread_bounds_along_axis(
                        self.bounds,
                        self.axis,
                        self.children.len(),
                        padding,
                    );
                    for (child, b) in self.children.iter_mut().zip(new_bounds) {
                        child.resize(b)?;
                    }

                    return Ok(RemoveResult::Removed);
                }
            }
        }

        Ok(RemoveResult::NotFound)
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        if self.children.len() < 2 {
            self.axis = axis;
            Ok(())
        } else {
            Err(Error::CannotSplitAlreadySplitContainer)
        }
    }

    pub fn resize_window(
        &mut self,
        target: WindowId,
        direction: Direction,
        padding: f64,
    ) -> Result<()> {
        let i = self
            .children
            .iter()
            .position(|c| c.contains_window(target))
            .ok_or(Error::WindowNotFound)?;

        if !self.axis.can_resize_in_direction(direction) {
            return self.children[i].resize_window(target, direction, padding);
        }

        let n = if i == 0 { 1 } else { i - 1 };
        let (left, right) = self.children.split_at_mut(i.max(n));
        let (a, b) = (&mut left[i.min(n)], &mut right[0]);

        Self::resize_at_split(self.axis, a, b, direction, padding)
    }

    fn resize_at_split(
        axis: Axis,
        a: &mut Container,
        b: &mut Container,
        direction: Direction,
        padding: f64,
    ) -> Result<()> {
        use Axis::*;
        use Direction::*;
        use window_manager::{MIN_WINDOW_SIZE, RESIZE_AMOUNT};

        let (first_bounds, second_bounds) = (a.bounds(), b.bounds());

        let delta = match direction {
            Left | Up => -RESIZE_AMOUNT,
            Right | Down => RESIZE_AMOUNT,
        };

        let midpoint = match axis {
            Vertical => first_bounds.y + first_bounds.height + padding / 2.0,
            Horizontal => first_bounds.x + first_bounds.width + padding / 2.0,
        };

        let new_midpoint = match axis {
            Vertical => (midpoint + delta)
                .max(first_bounds.y + MIN_WINDOW_SIZE)
                .min(first_bounds.y + first_bounds.height + second_bounds.height - MIN_WINDOW_SIZE),
            Horizontal => (midpoint + delta)
                .max(first_bounds.x + MIN_WINDOW_SIZE)
                .min(first_bounds.x + first_bounds.width + second_bounds.width - MIN_WINDOW_SIZE),
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

    // This is not a valid `Container` since the child bounds are wrong. It only
    // servers to be used in tests that are not checking correctness of bounds.
    #[cfg(test)]
    pub fn dummy(axis: Axis, window_ids: &[WindowId]) -> Container {
        use crate::container::tests::dummy_bounds;

        let children = window_ids.iter().map(Leaf::dummy).collect();
        Container::Split(Split::new(dummy_bounds(), axis, 0.0, children))
    }
}
