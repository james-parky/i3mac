use crate::error::{Error, Result};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

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

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
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
    pub fn window_bounds(&self) -> Option<Bounds> {
        match self {
            Container::Leaf {
                bounds, padding, ..
            } => Some(bounds.with_pad(*padding)),
            _ => None,
        }
    }

    fn add_window_to_empty(&mut self, window: Window, padding: f64) -> Result<()> {
        if let Self::Empty { bounds } = self {
            *self = Self::Split {
                bounds: *bounds,
                axis: Axis::default(),
                padding,
                children: vec![Self::Leaf {
                    bounds: *bounds,
                    padding,
                    window,
                }],
            };

            Ok(())
        } else {
            // TODO: proper error
            Err(Error::CannotAddWindowToLeaf)
        }
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
        if let Self::Split {
            bounds,
            axis,
            children,
            ..
        } = self
        {
            let num_new_children = children.len() + 1;
            let new_children_bounds =
                spread_bounds_along_axis(*bounds, *axis, num_new_children, padding);

            for (i, child) in children.iter().enumerate() {
                let width_ok = new_children_bounds[i].width >= child.min_width();
                let height_ok = new_children_bounds[i].height >= child.min_height();
                if !width_ok || !height_ok {
                    return Err(Error::CannotFitWindow);
                }
            }

            // Also check new window
            let last_bounds = new_children_bounds.last().unwrap();
            if last_bounds.width < window.min_width || last_bounds.height < window.min_height {
                return Err(Error::CannotFitWindow);
            }

            children.push(Container::Leaf {
                bounds: new_children_bounds[num_new_children - 1],
                padding,
                window,
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

                *self = Container::Split {
                    bounds: saved_bounds,
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

    pub(super) fn find_window(&self, target: WindowId) -> Option<WindowId> {
        match self {
            Self::Leaf { window, .. } if window.id == target => Some(window.id),
            Self::Split { children, .. } => {
                children.iter().find_map(|child| child.find_window(target))
            }
            _ => None,
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
            Self::Leaf {
                window,
                bounds,
                padding,
            } => HashMap::from([(window.id, bounds.with_pad(*padding))]),
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
                        bounds: self.get_bounds(),
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
        matches!(self, Self::Leaf{window,..} if window.id == target)
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
            // Try to remove child directly
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
                        child.resize(b, padding)?;
                    }
                }

                if let Container::Leaf { window, .. } = removed {
                    return Ok(Some(window.id));
                } else {
                    unreachable!()
                }
            }

            // Recurse into children
            for child in children.iter_mut() {
                if let Some(id) = child.remove_window(window_id, padding)? {
                    children.retain(|c| !matches!(c, Container::Empty { .. }));
                    return Ok(Some(id));
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
    ) -> Result<Option<WindowId>> {
        match self {
            Self::Empty { .. } => Ok(None),
            Self::Leaf { .. } => self.remove_window_from_leaf(window_id),
            Self::Split { .. } => self.remove_window_from_split(window_id, padding),
        }
    }

    fn resize_children(&mut self, new_bounds: Bounds, padding: f64) -> Result<()> {
        if let Self::Split { children, axis, .. } = self {
            let new_children_bounds =
                spread_bounds_along_axis(new_bounds, *axis, children.len(), padding);

            for (child, child_bounds) in children.iter_mut().zip(new_children_bounds) {
                child.resize(child_bounds, padding)?;
            }
        }

        Ok(())
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
    fn resize(&mut self, new_bounds: Bounds, padding: f64) -> Result<()> {
        match self {
            Self::Empty { bounds, .. } => *bounds = new_bounds,
            Self::Leaf { bounds, .. } => *bounds = new_bounds,
            Self::Split {
                bounds,
                children,
                axis,
                ..
            } => {
                *bounds = new_bounds;

                let child_bounds =
                    spread_bounds_along_axis(new_bounds, *axis, children.len(), padding);
                for (child, cb) in children.iter_mut().zip(child_bounds) {
                    child.resize(cb, padding)?;
                }
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
            // TODO: better error
            Self::Leaf { .. } => Err(Error::WindowNotFound),
            Self::Split { children, axis, .. } => {
                let child = children.iter().position(|c| c.contains_window(window_id));

                if let Some(i) = child {
                    if axis.can_resize_in_direction(direction) {
                        self.resize_at_split(i, direction, amount, padding)
                    } else {
                        children[i].resize_window(window_id, direction, amount, padding)
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
        if let Container::Split { children, .. } = self {
            let grow_idx = focused_idx;
            let shrink_idx = if focused_idx == 0 { 1 } else { focused_idx - 1 };
            let new_grow_bounds = children[grow_idx].get_bounds().grow(direction, amount);
            let new_shrink_bounds = children[shrink_idx]
                .get_bounds()
                .shrink(direction.opposite(), amount);

            children[grow_idx].resize(new_grow_bounds, padding)?;
            children[shrink_idx].resize(new_shrink_bounds, padding)?;
        }
        Ok(())
    }

    fn contains_window(&self, search: WindowId) -> bool {
        match self {
            Self::Empty { .. } => false,
            Self::Leaf { window, .. } => window.id == search,
            Self::Split { children, .. } => {
                children.iter().any(|child| child.contains_window(search))
            }
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

fn spread_bounds_along_axis(original: Bounds, axis: Axis, n: usize, padding: f64) -> Vec<Bounds> {
    match axis {
        Axis::Horizontal => {
            let total_inner_gap = (n - 1) as f64 * (padding); // half padding between children
            let available_width = original.width - 2.0 * padding - total_inner_gap;
            let child_width = available_width / (n as f64);

            (0..n)
                .map(|i| {
                    let x = original.x + padding + i as f64 * (child_width + padding);
                    Bounds {
                        x,
                        y: original.y + padding,
                        width: child_width,
                        height: original.height - 2.0 * padding,
                    }
                })
                .collect()
        }
        Axis::Vertical => {
            let total_inner_gap = (n - 1) as f64 * padding;
            let available_height = original.height - 2.0 * padding - total_inner_gap;
            let child_height = available_height / (n as f64);

            (0..n)
                .map(|i| {
                    let y = original.y + padding + i as f64 * (child_height + padding);
                    Bounds {
                        x: original.x + padding,
                        y,
                        width: original.width - 2.0 * padding,
                        height: child_height,
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_graphics::{Bounds, WindowId};

    const EPSILON: f64 = 1e-10;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn dummy_bounds() -> Bounds {
        Bounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    fn dummy_window(id: WindowId) -> Window {
        Window {
            id,
            min_width: 100.0,
            min_height: 100.0,
        }
    }

    fn dummy_empty() -> Container {
        Container::Empty {
            bounds: dummy_bounds(),
        }
    }

    fn dummy_leaf(window_id: WindowId) -> Container {
        Container::Leaf {
            bounds: dummy_bounds(),
            padding: 0.0,
            window: dummy_window(window_id),
        }
    }

    // This is not a valid `Container` since the child bounds are wrong. It only
    // servers to be used in tests that are not checking correctness of bounds.
    fn dummy_split(axis: Axis, window_ids: &[WindowId]) -> Container {
        Container::Split {
            bounds: dummy_bounds(),
            padding: 0.0,
            axis,
            children: window_ids.iter().map(|id| dummy_leaf(*id)).collect(),
        }
    }

    #[test]
    fn get_bounds() {
        let empty = dummy_empty();
        let leaf = dummy_leaf(WindowId::from(1u32));
        let split = dummy_split(Axis::default(), &[WindowId::from(1u32)]);

        assert_eq!(empty.get_bounds(), dummy_bounds());
        assert_eq!(leaf.get_bounds(), dummy_bounds());
        assert_eq!(split.get_bounds(), dummy_bounds());
    }

    #[test]
    fn contains_window() {
        assert!(!dummy_empty().contains_window(WindowId::from(1u32)));

        let target = WindowId::from(1u32);
        let leaf_with = dummy_leaf(target);
        let leaf_without = dummy_leaf(WindowId::from(2u32));
        let split_with = dummy_split(Axis::Horizontal, &[target]);
        let split_without = dummy_split(Axis::Horizontal, &[WindowId::from(2u32)]);

        assert!(leaf_with.contains_window(target));
        assert!(!leaf_without.contains_window(target));
        assert!(split_with.contains_window(target));
        assert!(!split_without.contains_window(target));
    }

    #[test]
    fn getting_window_bounds_from_non_leaf_is_none() {
        let empty = dummy_empty();
        assert!(empty.window_bounds().is_none());

        let split = dummy_split(Axis::default(), &[]);
        assert!(split.window_bounds().is_none());
    }

    #[test]
    fn leaf_window_bounds_includes_correct_padding() {
        let container_bounds = dummy_bounds();
        const PADDING: f64 = 10.0;
        let exp_window_bounds = Bounds {
            height: container_bounds.height - 20.0,
            width: container_bounds.width - 20.0,
            x: container_bounds.x + PADDING,
            y: container_bounds.y + PADDING,
        };

        let container = Container::Leaf {
            bounds: container_bounds,
            padding: PADDING,
            window: dummy_window(WindowId::from(1u32)),
        };

        assert_eq!(container.window_bounds().unwrap(), exp_window_bounds);
    }

    #[test]
    fn add_window_to_empty_creates_leaf() {
        let mut container = dummy_empty();
        let window_id = WindowId::from(1u32);

        assert!(container.add_window(dummy_window(window_id), 10.0).is_ok());

        match container {
            Container::Split { children, .. } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Container::Leaf {
                        window,
                        padding,
                        bounds,
                    } => {
                        assert_eq!(window.id, window_id);
                        assert_eq!(*padding, 10.0);
                        assert_eq!(*bounds, dummy_bounds());
                    }
                    _ => panic!("Expected a leaf inside split"),
                }
            }
            _ => panic!("Expected container to become a split"),
        }
    }

    #[test]
    fn add_window_to_split_adds_new_leaf() {
        let container_bounds = dummy_bounds();
        let mut container = dummy_empty();

        // The split should be horizontal with each leaf taking half the space.
        let exp_first_leaf_bounds = Bounds {
            height: container_bounds.height,
            width: container_bounds.width / 2.0,
            x: 0.0,
            y: 0.0,
        };
        let exp_second_leaf_bounds = Bounds {
            height: container_bounds.height,
            width: container_bounds.width / 2.0,
            x: container_bounds.width / 2.0,
            y: 0.0,
        };

        let first_id = WindowId::from(1u32);
        let second_id = WindowId::from(2u32);

        const PADDING: f64 = 0.0;

        container
            .add_window(dummy_window(first_id), PADDING)
            .unwrap();
        container
            .add_window(dummy_window(second_id), PADDING)
            .unwrap();

        match container {
            Container::Split { children, .. } => {
                assert_eq!(children.len(), 2);
                let ids: Vec<_> = children
                    .iter()
                    .map(|c| {
                        if let Container::Leaf { window, .. } = c {
                            window.id
                        } else {
                            WindowId::from(0u32)
                        }
                    })
                    .collect();
                assert!(ids.contains(&first_id));
                assert!(ids.contains(&second_id));
                assert_eq!(children[0].get_bounds(), exp_first_leaf_bounds);
                assert_eq!(children[1].get_bounds(), exp_second_leaf_bounds);
            }
            _ => panic!("Expected container to be a split"),
        }
    }

    #[test]
    fn add_window_to_leaf_errors() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let result = container.add_window(dummy_window(WindowId::from(2u32)), 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn splitting_empty_errors() {
        let mut container = dummy_empty();
        assert!(container.split(Axis::Vertical).is_err())
    }

    #[test]
    fn splitting_split_with_many_children_errors() {
        // This container could not exist because the bounds are wrong
        let mut container = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );
        assert!(container.split(Axis::Vertical).is_err());
    }

    #[test]
    fn splitting_split_with_one_child_changes_axis() {
        use Axis::*;

        for (starting_axis, change_axis, exp_axis) in [
            (Horizontal, Vertical, Vertical),
            (Horizontal, Horizontal, Horizontal),
            (Vertical, Horizontal, Horizontal),
            (Vertical, Vertical, Vertical),
        ] {
            let mut container = dummy_split(starting_axis, &[WindowId::from(1u32)]);
            container.split(change_axis).unwrap();
            assert!(
                matches!(container, Container::Split { axis, children, .. } if axis == exp_axis && children.len() == 1)
            );
        }
    }

    #[test]
    fn splitting_leaf_converts_to_split_with_same_bounds() {
        let mut container = dummy_leaf(WindowId::from(1u32));

        // The same as above, but we can't clone it since containers contain a
        // Vec
        let leaf = dummy_leaf(WindowId::from(1u32));

        container.split(Axis::Vertical).unwrap();

        assert!(matches!(container, Container::Split{
            bounds, children, axis, padding
        } if bounds == dummy_bounds()
            && children == vec![leaf]
            && axis == Axis::Vertical
            && padding == 0.0
        ));
    }

    #[test]
    fn find_window_empty_is_none() {
        let container = dummy_empty();
        assert!(container.find_window(WindowId::from(0u32)).is_none());
    }

    #[test]
    fn find_window_leaf() {
        let target = WindowId::from(1u32);
        let container = dummy_leaf(target);

        assert!(container.find_window(target).is_some());
        assert!(container.find_window(WindowId::from(18u32)).is_none());
    }

    #[test]
    fn find_window_split() {
        let target = WindowId::from(1u32);
        let container = dummy_split(Axis::default(), &[target]);

        assert!(container.find_window(target).is_some());
        assert!(container.find_window(WindowId::from(18u32)).is_none());
    }

    #[test]
    fn window_ids_empty() {
        let container = dummy_empty();
        assert!(container.window_ids().is_empty());
    }

    #[test]
    fn window_ids_leaf() {
        let container = dummy_leaf(WindowId::from(1u32));
        assert_eq!(
            container.window_ids(),
            HashSet::from([WindowId::from(1u32)])
        );
    }

    #[test]
    fn window_ids_split() {
        let window_ids = [WindowId::from(1u32), WindowId::from(2u32)];
        let container = dummy_split(Axis::default(), &window_ids);
        assert_eq!(container.window_ids(), HashSet::from(window_ids));
    }

    #[test]
    fn remove_window_from_leaf_non_existent() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let target = WindowId::from(2u32);
        assert!(container.remove_window_from_leaf(target).unwrap().is_none());
    }

    #[test]
    fn remove_window_from_leaf_existent() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let target = WindowId::from(1u32);
        assert!(
            container
                .remove_window_from_leaf(target)
                .unwrap()
                .is_some_and(|id| id == target)
        );
    }

    #[test]
    fn is_parent_leaf_non_leaf() {
        let empty = dummy_empty();
        let split = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );

        assert!(!empty.is_parent_leaf(WindowId::from(1u32)));
        assert!(!split.is_parent_leaf(WindowId::from(2u32)));
    }

    #[test]
    fn is_parent_leaf_leaf() {
        let target = WindowId::from(1u32);
        let parent = dummy_leaf(target);
        let non_parent = dummy_leaf(WindowId::from(2u32));

        assert!(parent.is_parent_leaf(target));
        assert!(!non_parent.is_parent_leaf(target));
    }

    #[test]
    fn remove_window_empty() {
        assert!(
            dummy_empty()
                .remove_window(WindowId::from(1u32), 0.0)
                .unwrap()
                .is_none()
        )
    }

    #[test]
    fn remove_window_leaf_target_exists() {
        let target = WindowId::from(1u32);
        let mut leaf = dummy_leaf(target);

        let res = leaf.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(leaf, Container::Empty { .. }))
    }

    #[test]
    fn remove_window_leaf_target_does_not_exist() {
        let target = WindowId::from(1u32);
        let mut leaf = dummy_leaf(WindowId::from(2u32));

        let res = leaf.remove_window(target, 0.0).unwrap();

        assert!(res.is_none());
        assert!(matches!(leaf, Container::Leaf { window,.. } if window.id == WindowId::from(2u32)));
    }

    #[test]
    fn remove_window_split_target_does_not_exist() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(Axis::default(), &[WindowId::from(2u32)]);

        let res = split.remove_window(target, 0.0).unwrap();
        assert!(res.is_none())
    }

    #[test]
    fn remove_window_split_target_exists_only_child() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(Axis::default(), &[target]);

        let res = split.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Empty { .. }))
    }

    #[test]
    fn remove_window_split_target_exists_two_children() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );

        let res = split.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Split { children, .. }
            if children == vec![Container::Leaf {
                bounds: split.get_bounds(),
                padding: 0.0,
                window: dummy_window(WindowId::from(2u32)),
            }]
        ))
    }

    #[test]
    fn remove_window_split_target_exists_many_children() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(
            Axis::default(),
            &[
                WindowId::from(1u32),
                WindowId::from(2u32),
                WindowId::from(3u32),
            ],
        );

        let res = split.remove_window(target, 0.0).unwrap();

        let exp_child_bounds =
            spread_bounds_along_axis(split.get_bounds(), Axis::default(), 2, 0.0);

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Split { children, .. }
            if children == vec![
                Container::Leaf {
                    bounds: exp_child_bounds[0],
                    padding: 0.0,
                    window:dummy_window(WindowId::from(2u32))
                },
                Container::Leaf {
                    bounds:exp_child_bounds[1],
                    padding: 0.0,
                    window:dummy_window(WindowId::from(3u32))
                }
            ]
        ))
    }

    #[test]
    fn get_leaf_of_window_mut_empty() {
        let mut container = dummy_empty();
        let target = WindowId::from(1u32);

        assert!(container.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn get_leaf_of_window_mut_leaf() {
        let target = WindowId::from(1u32);
        let mut is_parent = dummy_leaf(target);
        let mut not_parent = dummy_leaf(WindowId::from(2u32));

        assert!(is_parent.parent_leaf_of_window_mut(target).is_some_and(
            |leaf| matches!(leaf, Container::Leaf { window,.. } if window.id == target)
        ));

        assert!(not_parent.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn get_leaf_of_window_split() {
        let target = WindowId::from(1u32);
        let mut exists = dummy_split(Axis::default(), &[WindowId::from(2u32), target]);
        let mut doest_not_exist = dummy_split(
            Axis::default(),
            &[WindowId::from(2u32), WindowId::from(3u32)],
        );

        assert!(exists.parent_leaf_of_window_mut(target).is_some());
        assert!(doest_not_exist.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn spread_bounds_along_axis_horizontal() {
        let original = dummy_bounds();

        for &padding in &[0.0, 5.0, 10.0, 17.5] {
            for n in 1usize..=8 {
                let out = spread_bounds_along_axis(original, Axis::Horizontal, n, padding);
                assert_eq!(out.len(), n);

                let total_inner_gap = (n - 1) as f64 * padding;
                let available_width = original.width - 2.0 * padding - total_inner_gap;
                let child_width = available_width / n as f64;
                let expected_height = original.height - 2.0 * padding;

                // The first child starts after the correct padding
                assert!(approx(out[0].x, original.x + padding));
                assert!(approx(out[0].y, original.y + padding));

                // The last child has the correct amount of padding after it
                let last = &out[n - 1];
                assert!(approx(
                    last.x + last.width,
                    original.x + original.width - padding
                ));

                // Each inner child's bounds are correct
                for (i, b) in out.iter().enumerate() {
                    assert!(approx(b.width, child_width));
                    assert!(approx(b.height, expected_height));
                    assert!(approx(b.y, original.y + padding));

                    let expected_x = original.x + padding + i as f64 * (child_width + padding);
                    assert!(approx(b.x, expected_x));
                }

                // The difference between child positions is the padding
                for i in 1..n {
                    let prev = &out[i - 1];
                    let cur = &out[i];
                    assert!(approx(cur.x - (prev.x + prev.width), padding));
                }

                // Validate that the bounds aren't overlapping (bar some margin
                // for error)
                for i in 1..n {
                    assert!(out[i].x >= out[i - 1].x + out[i - 1].width - EPSILON);
                }

                // Full available width is spanned
                let covered: f64 = out.iter().map(|b| b.width).sum();
                assert!(approx(covered, available_width));
            }
        }
    }

    #[test]
    fn spread_bounds_along_axis_vertical() {
        let original = dummy_bounds();

        for &padding in &[0.0, 3.0, 8.0, 20.0] {
            for n in 1usize..=8 {
                let out = spread_bounds_along_axis(original, Axis::Vertical, n, padding);

                assert_eq!(out.len(), n);

                let total_inner_gap = (n - 1) as f64 * padding;
                let available_height = original.height - 2.0 * padding - total_inner_gap;
                let child_height = available_height / n as f64;
                let expected_width = original.width - 2.0 * padding;

                assert!(approx(out[0].y, original.y + padding));
                assert!(approx(out[0].x, original.x + padding));

                let last = &out[n - 1];
                assert!(approx(
                    last.y + last.height,
                    original.y + original.height - padding
                ));

                for (i, b) in out.iter().enumerate() {
                    assert!(approx(b.height, child_height));
                    assert!(approx(b.width, expected_width));
                    assert!(approx(b.x, original.x + padding));

                    let expected_y = original.y + padding + i as f64 * (child_height + padding);
                    assert!(approx(b.y, expected_y));
                }

                for i in 1..n {
                    let prev = &out[i - 1];
                    let cur = &out[i];
                    assert!(approx(cur.y - (prev.y + prev.height), padding));
                }

                for i in 1..n {
                    assert!(out[i].y >= out[i - 1].y + out[i - 1].height - EPSILON);
                }

                let covered: f64 = out.iter().map(|b| b.height).sum();
                assert!(approx(covered, available_height));
            }
        }
    }

    #[test]
    fn n_equals_one_fills_inner_area() {
        let original = dummy_bounds();

        for &padding in &[0.0, 5.0, 25.0] {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                let out = spread_bounds_along_axis(original, axis, 1, padding);
                let b = &out[0];

                assert!(approx(b.x, original.x + padding));
                assert!(approx(b.y, original.y + padding));
                assert!(approx(b.width, original.width - 2.0 * padding));
                assert!(approx(b.height, original.height - 2.0 * padding));
            }
        }
    }

    #[test]
    fn spread_bounds_along_axis_symmetry() {
        let original = Bounds {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 800.0,
        };

        let padding = 12.0;
        let n = 4;

        let h = spread_bounds_along_axis(original, Axis::Horizontal, n, padding);

        let transposed = Bounds {
            x: original.x,
            y: original.y,
            width: original.height,
            height: original.width,
        };

        let v = spread_bounds_along_axis(transposed, Axis::Vertical, n, padding);

        for (bh, bv) in h.iter().zip(v.iter()) {
            assert!(approx(bh.width, bv.height));
            assert!(approx(bh.height, bv.width));
        }
    }
}
