use crate::{
    error::{Error, Result},
    window::Window,
};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Default, Copy, Clone, Hash)]
pub enum Axis {
    Vertical,
    #[default]
    Horizontal,
}

impl Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "Vertical"),
            Self::Horizontal => write!(f, "Horizontal"),
        }
    }
}

impl Axis {
    fn can_resize_in_direction(&self, direction: Direction) -> bool {
        matches!(
            (self, direction),
            (Axis::Horizontal, Direction::Left)
                | (Axis::Horizontal, Direction::Right)
                | (Axis::Vertical, Direction::Up)
                | (Axis::Vertical, Direction::Down)
        )
    }
}

#[derive(Debug)]
pub(super) enum Container {
    Empty {
        bounds: Bounds,
    },
    Leaf {
        bounds: Bounds,
        window: Window,
    },
    Split {
        bounds: Bounds,
        axis: Axis,
        children: Vec<Container>,
    },
}

impl Container {
    fn add_window_to_empty(
        &mut self,
        cg_window: core_graphics::Window,
        padding: f64,
    ) -> Result<()> {
        if let Self::Empty { bounds } = self {
            let window_bounds = bounds.with_pad(padding);
            let mut window = Window::try_new(cg_window, window_bounds)?;
            window.init()?;

            *self = Self::Split {
                bounds: *bounds,
                axis: Axis::default(),
                children: vec![Self::Leaf {
                    bounds: *bounds,
                    window,
                }],
            };

            Ok(())
        } else {
            // TODO: proper error
            Err(Error::CannotAddWindowToLeaf)
        }
    }

    // To add a window to a split container:
    //  1. Create the new window and add it to the split's children.
    //  2. Spread the containers bounds across the now N children.
    //  3. Resize all children using those new bounds.
    fn add_window_to_split(
        &mut self,
        cg_window: core_graphics::Window,
        padding: f64,
    ) -> Result<()> {
        if let Self::Split {
            bounds,
            children,
            axis: direction,
        } = self
        {
            let num_new_children = children.len() + 1;
            let new_children_bounds =
                spread_bounds_in_direction(*bounds, *direction, num_new_children, padding);

            let window_bounds = new_children_bounds[num_new_children - 1];
            let mut new_window = Window::try_new(cg_window, window_bounds)?;
            new_window.init()?;

            children.push(Container::Leaf {
                bounds: window_bounds,
                window: new_window,
            });

            for (child, new_bounds) in children.iter_mut().zip(new_children_bounds) {
                child.resize(new_bounds, padding)?;
            }

            Ok(())
        } else {
            Err(Error::ExpectedSplitContainer)
        }
    }

    // Rules for adding a window to a container:
    //  - If a container is empty, create a default split and add the new window
    //    as a single child.
    //  - If a container is a leaf, return an error.
    //  - If a container is a split, add a new child, and adjust the bounds of
    //    the existing children. This requires adjusting the bounds of all
    //    existing children in said split, recursively.
    pub fn add_window(&mut self, cg_window: core_graphics::Window, padding: f64) -> Result<()> {
        match self {
            Self::Empty { .. } => self.add_window_to_empty(cg_window, padding),
            Self::Leaf { .. } => Err(Error::CannotAddWindowToLeaf),
            Self::Split { .. } => self.add_window_to_split(cg_window, padding),
        }
    }

    pub fn split(&mut self, direction: Axis) -> Result<()> {
        match self {
            Self::Empty { .. } => Err(Error::CannotSplitEmptyContainer),
            Self::Leaf { bounds, .. } => {
                let saved_bounds = *bounds;
                let temp = Self::Empty {
                    bounds: saved_bounds,
                };
                let old_self = std::mem::replace(self, temp);

                if let Self::Leaf { window, .. } = old_self {
                    *self = Self::Split {
                        bounds: saved_bounds,
                        axis: direction,
                        children: vec![Self::Leaf {
                            bounds: saved_bounds,
                            window,
                        }],
                    };
                    Ok(())
                } else {
                    unreachable!()
                }
            }
            Self::Split {
                children,
                axis: current_split,
                ..
            } if children.len() < 2 => {
                *current_split = direction;
                Ok(())
            }
            Self::Split { .. } => Err(Error::CannotSplitAlreadySplitContainer),
        }
    }

    pub fn all_windows(&self) -> Vec<&Window> {
        match self {
            Self::Empty { .. } => Vec::new(),
            Self::Leaf { window, .. } => vec![window],
            Self::Split { children, .. } => {
                let mut windows = Vec::new();
                for child in children {
                    windows.extend(child.all_windows());
                }
                windows
            }
        }
    }

    pub(super) fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        match self {
            Self::Leaf { window, .. } if window.cg().number() == window_id => Some(window),
            Self::Split { children, .. } => children
                .iter()
                .find_map(|child| child.find_window(window_id)),
            _ => None,
        }
    }

    pub(super) fn window_ids(&self) -> HashSet<WindowId> {
        match self {
            Self::Empty { .. } => HashSet::new(),
            Self::Leaf { window, .. } => HashSet::from([window.cg().number()]),
            Self::Split { children, .. } => children
                .iter()
                .flat_map(|child| child.window_ids())
                .collect(),
        }
    }

    pub(super) fn windows_mut(&mut self) -> HashSet<&mut Window> {
        match self {
            Self::Empty { .. } => HashSet::new(),
            Self::Leaf { window, .. } => HashSet::from([window]),
            Self::Split { children, .. } => children
                .iter_mut()
                .flat_map(|child| child.windows_mut())
                .collect(),
        }
    }

    fn remove_window_from_leaf(&mut self, window_id: WindowId) -> Result<Option<Window>> {
        if let Self::Leaf { window, .. } = self {
            if window.cg().number() == window_id {
                let old = std::mem::replace(
                    self,
                    Self::Empty {
                        bounds: Bounds::default(),
                    },
                );
                if let Self::Leaf { window, .. } = old {
                    Ok(Some(window))
                } else {
                    unreachable!()
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn is_parent_leaf(&self, window_id: WindowId) -> bool {
        matches!(self, Self::Leaf{window,..} if window.cg().number() == window_id)
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty { .. })
    }

    fn remove_window_from_split(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<Option<Window>> {
        if let Self::Split {
            children,
            bounds,
            axis,
        } = self
        {
            if let Some(i) = children
                .iter()
                .position(|child| child.is_parent_leaf(window_id))
            {
                let removed_child = children.remove(i);
                children.retain(|c| !c.is_empty());

                if children.is_empty() {
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
        } else {
            Ok(None)
        }
    }

    pub(super) fn remove_window(
        &mut self,
        window_id: WindowId,
        padding: f64,
    ) -> Result<Option<Window>> {
        match self {
            Self::Empty { .. } => Ok(None),
            Self::Leaf { .. } => self.remove_window_from_leaf(window_id),
            Self::Split { .. } => self.remove_window_from_split(window_id, padding),
        }
    }

    fn resize_children(&mut self, new_bounds: Bounds, padding: f64) -> Result<()> {
        if let Self::Split { children, axis, .. } = self {
            let new_children_bounds =
                spread_bounds_in_direction(new_bounds, *axis, children.len(), padding);

            for (child, child_bounds) in children.iter_mut().zip(new_children_bounds) {
                child.resize(child_bounds, padding)?;
            }
        }

        Ok(())
    }

    pub fn parent_leaf_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Self> {
        match self {
            Self::Leaf { window, .. } if window.cg().number() == window_id => Some(self),
            Self::Split { children, .. } => {
                for child in children {
                    if let Some(parent) = child.parent_leaf_of_window_mut(window_id) {
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
    fn resize(&mut self, new_bounds: Bounds, padding: f64) -> Result<()> {
        match self {
            Self::Empty { bounds } => {
                *bounds = new_bounds;
            }
            Self::Leaf { bounds, window } => {
                *bounds = new_bounds;
                window.update_bounds(new_bounds)?;
            }
            Self::Split { bounds, .. } => {
                *bounds = new_bounds;
                self.resize_children(new_bounds, padding)?;
            }
        }

        Ok(())
    }

    pub fn resize_window(
        &mut self,
        window_id: WindowId,
        direction: Direction,
        amount: f64,
        padding: f64,
    ) -> Result<()> {
        match self {
            Self::Empty { .. } => Err(Error::WindowNotFound),
            Self::Leaf { window, .. } if window.cg().number() == window_id => {
                Err(Error::CannotResizeRoot)
            }
            Self::Leaf { .. } => Err(Error::WindowNotFound),
            Self::Split { children, axis, .. } => {
                let child = children.iter().position(|c| c.contains_window(window_id));

                if let Some(i) = child {
                    if axis.can_resize_in_direction(direction) {
                        self.resize_at_split(i, direction, amount, padding)
                    } else {
                        match children[i].resize_window(window_id, direction, amount, padding) {
                            Ok(_) => Ok(()),
                            Err(Error::CannotResizeRoot) => {
                                self.resize_child_container(i, direction, amount, padding)
                            }
                            Err(e) => Err(e),
                        }
                    }
                } else {
                    Err(Error::WindowNotFound)
                }
            }
        }
    }

    // New function to resize an entire child container
    fn resize_child_container(
        &mut self,
        child_idx: usize,
        direction: Direction,
        amount: f64,
        padding: f64,
    ) -> Result<()> {
        if let Self::Split { axis, .. } = self {
            if axis.can_resize_in_direction(direction) {
                // Resize at this level, treating child_idx as the focused container
                self.resize_at_split(child_idx, direction, amount, padding)
            } else {
                // Still wrong direction, can't resize here
                Err(Error::CannotResizeRoot)
            }
        } else {
            Err(Error::CannotResizeRoot)
        }
    }

    fn resize_at_split(
        &mut self,
        focused_idx: usize,
        direction: Direction,
        amount: f64,
        padding: f64,
    ) -> Result<()> {
        if let Self::Split { children, axis, .. } = self {
            let at_start_edge = focused_idx == 0;
            let at_end_edge = focused_idx == children.len() - 1;

            let is_toward_start = matches!(
                (*axis, direction),
                (Axis::Horizontal, Direction::Left) | (Axis::Vertical, Direction::Up)
            );

            let is_toward_end = matches!(
                (*axis, direction),
                (Axis::Horizontal, Direction::Right) | (Axis::Vertical, Direction::Down)
            );

            let (grow_idx, shrink_idx, is_shrinking) = match (is_toward_start, is_toward_end) {
                (true, _) if at_start_edge => (focused_idx + 1, focused_idx, true),
                (true, _) => (focused_idx, focused_idx - 1, false),
                (_, true) if at_end_edge => (focused_idx - 1, focused_idx, true),
                (_, true) => (focused_idx, focused_idx + 1, false),
                _ => return Ok(()),
            };

            let is_grow_before_shrink = if is_shrinking {
                focused_idx < grow_idx
            } else {
                grow_idx < shrink_idx
            };

            let (grow_dir, shrink_dir) = if (is_shrinking && is_grow_before_shrink)
                || (!is_shrinking && !is_grow_before_shrink)
            {
                match *axis {
                    Axis::Vertical => (Direction::Up, Direction::Down),
                    Axis::Horizontal => (Direction::Left, Direction::Right),
                }
            } else {
                match *axis {
                    Axis::Vertical => (Direction::Down, Direction::Up),
                    Axis::Horizontal => (Direction::Right, Direction::Left),
                }
            };

            let new_grow_bounds = children[grow_idx].get_bounds().grow(grow_dir, amount);
            let new_shrink_bounds = children[shrink_idx].get_bounds().shrink(shrink_dir, amount);

            children[grow_idx].resize(new_grow_bounds, padding)?;
            children[shrink_idx].resize(new_shrink_bounds, padding)
        } else {
            Ok(())
        }
    }

    fn contains_window(&self, window_id: WindowId) -> bool {
        match self {
            Self::Empty { .. } => false,
            Self::Leaf { window, .. } => window.cg().number() == window_id,
            Self::Split { children, .. } => children
                .iter()
                .any(|child| child.contains_window(window_id)),
        }
    }

    fn get_bounds(&self) -> Bounds {
        match self {
            Self::Empty { bounds } => *bounds,
            Self::Leaf { bounds, .. } => *bounds,
            Self::Split { bounds, .. } => *bounds,
        }
    }
}

fn spread_bounds_in_direction(
    original: Bounds,
    direction: Axis,
    n: usize,
    padding: f64,
) -> Vec<Bounds> {
    match direction {
        Axis::Horizontal => {
            let total_gap = (2.0 * padding) + ((n - 1) as f64 * padding);
            let available_width = original.width - total_gap;
            let child_width = available_width / (n as f64);

            (0..n)
                .map(|i| {
                    let x = original.x + padding + (i as f64 * (child_width + padding));
                    Bounds {
                        x,
                        width: child_width,
                        y: original.y + padding,
                        height: original.height - (2.0 * padding),
                    }
                })
                .collect()
        }
        Axis::Vertical => {
            let total_gap = (2.0 * padding) + ((n - 1) as f64 * padding);
            let available_height = original.height - total_gap;
            let child_width = available_height / (n as f64);

            (0..n)
                .map(|i| {
                    let y = original.y + padding + (i as f64 * (child_width + padding));
                    Bounds {
                        x: original.x + padding,
                        width: child_width,
                        y,
                        height: original.height - (2.0 * padding),
                    }
                })
                .collect()
        }
    }
}
