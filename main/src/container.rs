use crate::{Error, window::Window};
use core_graphics::{Bounds, WindowId};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Debug, Default, Clone, Hash)]
pub enum Direction {
    Vertical,
    #[default]
    Horizontal,
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
        direction: Direction,
        children: Vec<Container>,
    },
}

struct CloseContext {
    container: Rc<RefCell<Container>>,
    window: Window,
}

impl Container {
    pub fn sibling_of(&self, window_id: WindowId) -> Option<WindowId> {
        match self {
            Self::Empty { .. } | Self::Leaf { .. } => None,
            Self::Split { children, .. } => {
                for (idx, child) in children.iter().enumerate() {
                    if let Self::Leaf { window, .. } = child {
                        if window.cg().number() == window_id {
                            if idx + 1 < children.len() {
                                return children[idx + 1].get_first_window();
                            } else if idx > 0 {
                                return children[idx - 1].get_first_window();
                            } else {
                                return None;
                            }
                        }
                    }
                }

                for child in children {
                    if child.contains_window(window_id) {
                        if let Some(sibling) = child.sibling_of(window_id) {
                            return Some(sibling);
                        }
                    }
                }
                None
            }
        }
    }
    fn get_first_window(&self) -> Option<WindowId> {
        match self {
            Self::Empty { .. } => None,
            Self::Leaf { window, .. } => Some(window.cg().number()),
            Self::Split { children, .. } => children.first().and_then(|c| c.get_first_window()),
        }
    }
    pub fn add_window(&mut self, cg_window: core_graphics::Window) -> crate::Result<()> {
        match self {
            Self::Leaf { .. } => return Err(Error::CannotAddWindowToLeaf),
            Self::Empty { bounds } => {
                let mut window = Window::try_new(cg_window, *bounds)?;
                window.init()?;
                *self = Self::Split {
                    bounds: *bounds,
                    direction: Direction::default(),
                    children: vec![Self::Leaf {
                        bounds: *bounds,
                        window,
                    }],
                };
            }
            Self::Split {
                bounds,
                direction,
                children,
            } => {
                let saved_bounds = *bounds;
                let n = children.len() + 1;

                let (sizes, positions) = match direction {
                    Direction::Horizontal => {
                        let widths = vec![saved_bounds.width / n as f64; n];
                        let mut xs = vec![saved_bounds.x];
                        for i in 1..n {
                            xs.push(xs[i - 1] + widths[i - 1]);
                        }
                        (widths, xs)
                    }
                    Direction::Vertical => {
                        let heights = vec![saved_bounds.height / n as f64; n];
                        let mut ys = vec![saved_bounds.y];
                        for i in 1..n {
                            ys.push(ys[i - 1] + heights[i - 1]);
                        }
                        (heights, ys)
                    }
                };

                for (i, child) in children.iter_mut().enumerate() {
                    let new_bounds = match direction {
                        Direction::Horizontal => Bounds {
                            x: positions[i],
                            width: sizes[i],
                            ..saved_bounds
                        },
                        Direction::Vertical => Bounds {
                            y: positions[i],
                            height: sizes[i],
                            ..saved_bounds
                        },
                    };

                    if let Self::Leaf { bounds, window } = child {
                        *bounds = new_bounds;
                        window.update_bounds(new_bounds)?;
                    }
                }

                let new_child_bounds = match direction {
                    Direction::Horizontal => Bounds {
                        x: positions[n - 1],
                        width: sizes[n - 1],
                        ..saved_bounds
                    },
                    Direction::Vertical => Bounds {
                        y: positions[n - 1],
                        height: sizes[n - 1],
                        ..saved_bounds
                    },
                };

                let mut new_window = Window::try_new(cg_window, new_child_bounds)?;
                new_window.init()?;

                children.push(Container::Leaf {
                    bounds: new_child_bounds,
                    window: new_window,
                });
            }
        }

        Ok(())
    }

    pub(super) fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        match self {
            Self::Leaf { window, .. } if window.cg().number() == window_id => Some(&window),
            Self::Split { children, .. } => children
                .iter()
                .find_map(|child| child.find_window(window_id)),
            _ => None,
        }
    }
    pub(super) fn find_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        match self {
            Self::Leaf { window, .. } if window.cg().number() == window_id => Some(window),
            Self::Split { children, .. } => children
                .iter_mut()
                .find_map(|child| child.find_window_mut(window_id)),
            _ => None,
        }
    }

    pub(super) fn cg_windows(&self) -> HashSet<&core_graphics::Window> {
        match &self {
            Self::Empty { .. } => HashSet::new(),
            Self::Leaf { window, .. } => HashSet::from([window.cg()]),
            Self::Split { children, .. } => children
                .iter()
                .flat_map(|child| child.cg_windows())
                .collect(),
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

    pub(super) fn remove_window(&mut self, window_id: WindowId) -> crate::Result<bool> {
        match self {
            Self::Empty { .. } => Ok(false),
            Self::Leaf { window, .. } if window.cg().number() == window_id => Ok(true),
            Self::Leaf { .. } => Ok(false),
            Self::Split {
                children, bounds, ..
            } => {
                let saved_bounds = bounds.clone();

                if let Some(i) = children.iter().position(|child| matches!(child, Self::Leaf{window,..} if window.cg().number() == window_id)) {
                    children.remove(i);
                    children.retain(|c| !matches!(c, Self::Empty {..}));

                    if children.is_empty() {
                        *self = Self::Empty { bounds: saved_bounds };
                    } else {
                        self.update_children_bounds(saved_bounds)?;
                    }
                    return Ok(true);
                }

                for child in children.iter_mut() {
                    if child.remove_window(window_id)? {
                        children.retain(|c| !matches!(c, Self::Empty { .. }));

                        if children.is_empty() {
                            *self = Self::Empty {
                                bounds: saved_bounds,
                            };
                        }
                        // else {
                        //     self.update_children_bounds(saved_bounds)?;
                        // }
                        return Ok(true);
                    }
                }

                Ok(false)
            }
        }
    }

    fn update_children_bounds(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        if let Self::Split {
            children,
            direction,
            ..
        } = self
        {
            let n = children.len();
            match direction {
                Direction::Horizontal => {
                    let widths = vec![new_bounds.width / n as f64; n];
                    let mut xs = vec![new_bounds.x];

                    for i in 1..n {
                        xs.push(xs[i - 1] + widths[i - 1]);
                    }

                    for (i, child) in children.iter_mut().enumerate() {
                        let child_bounds = Bounds {
                            x: xs[i],
                            width: widths[i],
                            ..new_bounds
                        };

                        child.update_bounds(child_bounds)?;
                    }
                }
                Direction::Vertical => {
                    let heights = vec![new_bounds.height / n as f64; n];
                    let mut ys = vec![new_bounds.y];

                    for i in 1..n {
                        ys.push(ys[i - 1] + heights[i - 1]);
                    }

                    for (i, child) in children.iter_mut().enumerate() {
                        let child_bounds = Bounds {
                            y: ys[i],
                            height: heights[i],
                            ..new_bounds
                        };

                        child.update_bounds(child_bounds)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn cleanup_children(&mut self) {
        if let Self::Split { children, .. } = self {
            children.retain(|child| !matches!(child, Self::Empty { .. }));
        }
    }
    pub(super) fn split(&mut self, direction: Direction) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Err(Error::CannotSplitEmptyContainer),
            Self::Leaf { bounds, .. } => {
                let old_bounds = *bounds;
                let temp = Self::Empty { bounds: old_bounds };
                let old_self = std::mem::replace(self, temp);

                if let Self::Leaf { window, .. } = old_self {
                    *self = Self::Split {
                        bounds: old_bounds,
                        direction,
                        children: vec![Self::Leaf {
                            bounds: old_bounds,
                            window,
                        }],
                    };
                    Ok(())
                } else {
                    unreachable!();
                }
            }

            Self::Split { .. } => Err(Error::CannotSplitAlreadySplitContainer),
        }
    }

    fn disable_all_observers(&mut self) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Ok(()),
            Self::Leaf { window, .. } => window.disable_observers(),
            Self::Split { children, .. } => {
                for child in children {
                    child.disable_all_observers()?;
                }
                Ok(())
            }
        }
    }

    fn enable_all_observers(&mut self) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Ok(()),
            Self::Leaf { window, .. } => window.enable_observers(),
            Self::Split { children, .. } => {
                for child in children {
                    child.enable_all_observers()?;
                }
                Ok(())
            }
        }
    }

    fn recalculate_layout(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        println!("recalculating layout...for {:?}", self);
        match self {
            Self::Empty { .. } => Ok(()),
            Self::Leaf { bounds, window, .. } => {
                *bounds = new_bounds;
                // window.update_bounds(new_bounds)
                let update_bounds = window.update_bounds_no_observer_update(new_bounds);
                println!("update bounds error: {update_bounds:?}");
                update_bounds
            }
            Self::Split {
                children,
                bounds,
                direction,
                ..
            } => {
                *bounds = new_bounds;

                let n = children.len();
                if n == 0 {
                    return Ok(());
                }

                println!("disabled obrservers at split level");
                for child in children.iter_mut() {
                    child.disable_all_observers()?;
                }

                let result = match direction {
                    Direction::Horizontal => {
                        let widths = vec![new_bounds.width / n as f64; n];
                        let mut xs = vec![new_bounds.x];
                        for i in 1..n {
                            xs.push(xs[i - 1] + widths[i - 1]);
                        }

                        for (i, child) in children.iter_mut().enumerate() {
                            let child_bounds = Bounds {
                                x: xs[i],
                                width: widths[i],
                                ..new_bounds
                            };
                            match child {
                                Container::Leaf { bounds, window } => {
                                    *bounds = child_bounds;
                                    window.update_bounds_no_observer_update(child_bounds)?;
                                }
                                Container::Split { .. } => {
                                    child.recalculate_layout_inner(child_bounds)?;
                                }
                                Container::Empty { .. } => {}
                            }
                        }
                        Ok(())
                    }
                    Direction::Vertical => {
                        let heights = vec![new_bounds.height / n as f64; n];
                        let mut ys = vec![new_bounds.y];
                        for i in 1..n {
                            ys.push(ys[i - 1] + heights[i - 1]);
                        }

                        for (i, child) in children.iter_mut().enumerate() {
                            let child_bounds = Bounds {
                                y: ys[i],
                                height: heights[i],
                                ..new_bounds
                            };
                            match child {
                                Container::Leaf { bounds, window } => {
                                    *bounds = child_bounds;
                                    window.update_bounds_no_observer_update(child_bounds)?;
                                }
                                Container::Split { .. } => {
                                    child.recalculate_layout_inner(child_bounds)?;
                                }
                                Container::Empty { .. } => {}
                            }
                        }
                        Ok(())
                    }
                };
                for child in children.iter_mut() {
                    child.disable_all_observers()?;
                }
                result
            }
        }
    }

    fn recalculate_layout_inner(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Ok(()),
            Self::Leaf { bounds, window, .. } => {
                *bounds = new_bounds;
                window.update_bounds_no_observer_update(new_bounds)
            }
            Self::Split {
                children,
                bounds,
                direction,
                ..
            } => {
                *bounds = new_bounds;
                let n = children.len();
                if n == 0 {
                    return Ok(());
                }

                match direction {
                    Direction::Horizontal => {
                        let widths = vec![new_bounds.width / n as f64; n];
                        let mut xs = vec![new_bounds.x];
                        for i in 1..n {
                            xs.push(xs[i - 1] + widths[i - 1]);
                        }
                        for (i, child) in children.iter_mut().enumerate() {
                            let child_bounds = Bounds {
                                x: xs[i],
                                width: widths[i],
                                ..new_bounds
                            };
                            child.recalculate_layout_inner(child_bounds)?;
                        }
                    }
                    Direction::Vertical => {
                        let heights = vec![new_bounds.height / n as f64; n];
                        let mut ys = vec![new_bounds.y];
                        for i in 1..n {
                            ys.push(ys[i - 1] + heights[i - 1]);
                        }
                        for (i, child) in children.iter_mut().enumerate() {
                            let child_bounds = Bounds {
                                y: ys[i],
                                height: heights[i],
                                ..new_bounds
                            };
                            child.recalculate_layout_inner(child_bounds)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    pub fn get_leaf_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Self> {
        match self {
            Self::Leaf { window, .. } if window.cg().number() == window_id => Some(self),
            Self::Split { children, .. } => {
                for child in children {
                    if let Some(parent) = child.get_leaf_of_window_mut(window_id) {
                        return Some(parent);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn get_parent_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Self> {
        if !matches!(self, Self::Split { .. }) {
            return None;
        }

        let has_matching_child = match self {
            Self::Split { children, .. } => children.iter().any(|child| {
                matches!(child, Self::Leaf { window, .. } if window.cg().number() == window_id)
            }),
            _ => false,
        };

        if has_matching_child {
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

    fn update_bounds(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        match self {
            Self::Empty { bounds } => {
                *bounds = new_bounds;
                Ok(())
            }
            Self::Leaf { bounds, window } => {
                *bounds = new_bounds;
                window.update_bounds(new_bounds)
            }
            // Self::Split { .. } => self.update_children_bounds(new_bounds),
            Self::Split { bounds, .. } => {
                let old_bounds = *bounds;
                *bounds = new_bounds;

                if old_bounds != new_bounds {
                    self.update_children_bounds(new_bounds)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn resize_window(
        &mut self,
        window_id: WindowId,
        direction: &core_graphics::Direction,
        amount: f64,
        container_bounds: Bounds,
    ) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Err(Error::WindowNotFound),
            Self::Leaf { window, .. } if window.cg().number() == window_id => {
                Err(Error::CannotResizeRoot)
            }
            Self::Leaf { .. } => Err(Error::WindowNotFound),
            Self::Split {
                children,
                bounds,
                direction: split_dir,
            } => {
                let saved_bounds = *bounds;

                let child = children.iter().position(|c| c.contains_window(window_id));

                if let Some(i) = child {
                    let can_resize = match (split_dir, &direction) {
                        (Direction::Horizontal, core_graphics::Direction::Left) => true,
                        (Direction::Horizontal, core_graphics::Direction::Right) => true,
                        (Direction::Vertical, core_graphics::Direction::Up) => true,
                        (Direction::Vertical, core_graphics::Direction::Down) => true,
                        _ => false,
                    };

                    if can_resize {
                        self.resize_at_split(i, &direction, amount, saved_bounds)
                    } else {
                        let child_bounds = children[i].get_bounds();

                        match children[i].resize_window(window_id, direction, amount, child_bounds)
                        {
                            Ok(_) => Ok(()),
                            Err(Error::CannotResizeRoot) => {
                                self.resize_child_container(i, &direction, amount, saved_bounds)
                            }
                            Err(e) => Err(e),
                        }
                        // children[i].resize_window(window_id, direction, amount, child_bounds)
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
        direction: &core_graphics::Direction,
        amount: f64,
        parent_bounds: Bounds,
    ) -> crate::Result<()> {
        if let Self::Split {
            children,
            direction: split_dir,
            ..
        } = self
        {
            let can_resize = match (split_dir, direction) {
                (Direction::Horizontal, core_graphics::Direction::Left)
                | (Direction::Horizontal, core_graphics::Direction::Right) => true,
                (Direction::Vertical, core_graphics::Direction::Up)
                | (Direction::Vertical, core_graphics::Direction::Down) => true,
                _ => false,
            };

            if can_resize {
                // Resize at this level, treating child_idx as the focused container
                self.resize_at_split(child_idx, direction, amount, parent_bounds)
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
        direction: &core_graphics::Direction,
        amount: f64,
        parent_bounds: Bounds,
    ) -> crate::Result<()> {
        if let Self::Split {
            children,
            direction: split_dir,
            ..
        } = self
        {
            let n = children.len();
            let saved_split_dir = split_dir.clone();

            // Determine which sibling(s) to shrink
            let (grow_idx, shrink_idx, is_shrinking) = match (&saved_split_dir, direction) {
                (Direction::Horizontal, core_graphics::Direction::Left) => {
                    // Grow left = take from left sibling
                    if focused_idx == 0 {
                        (focused_idx + 1, focused_idx, true)
                    } else {
                        (focused_idx, focused_idx - 1, false)
                    }
                }
                (Direction::Horizontal, core_graphics::Direction::Right) => {
                    // Grow right = take from right sibling
                    if focused_idx == n - 1 {
                        (focused_idx - 1, focused_idx, true)
                    } else {
                        (focused_idx, focused_idx + 1, false)
                    }
                }
                (Direction::Vertical, core_graphics::Direction::Up) => {
                    // Grow up = take from upper sibling
                    if focused_idx == 0 {
                        (focused_idx + 1, focused_idx, true)
                    } else {
                        (focused_idx, focused_idx - 1, false)
                    }
                }
                (Direction::Vertical, core_graphics::Direction::Down) => {
                    // Grow down = take from lower sibling
                    if focused_idx == n - 1 {
                        (focused_idx - 1, focused_idx, true)
                    } else {
                        (focused_idx, focused_idx + 1, false)
                    }
                }
                _ => return Ok(()), // Shouldn't happen
            };

            let grow_bounds = children[grow_idx].get_bounds();
            let shrink_bounds = children[shrink_idx].get_bounds();

            // Calculate new bounds
            match saved_split_dir {
                Direction::Horizontal => {
                    // Check if shrink child would become too small
                    if shrink_bounds.width - amount < 100.0 {
                        return Ok(()); // Don't resize if it would make window too small
                    }
                    // Calculate new bounds based on which window is growing/shrinking
                    if is_shrinking {
                        // Focused window is shrinking, sibling is growing
                        if focused_idx < grow_idx {
                            // Focused is left, next grows left
                            let new_shrink_bounds = Bounds {
                                width: shrink_bounds.width - amount,
                                ..shrink_bounds
                            };
                            let new_grow_bounds = Bounds {
                                x: grow_bounds.x - amount,
                                width: grow_bounds.width + amount,
                                ..grow_bounds
                            };
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                        } else {
                            // Focused is right, previous grows right
                            let new_grow_bounds = Bounds {
                                width: grow_bounds.width + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                x: shrink_bounds.x + amount,
                                width: shrink_bounds.width - amount,
                                ..shrink_bounds
                            };
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                        }
                    } else {
                        // Normal growth - focused window grows
                        if grow_idx < shrink_idx {
                            // Growing right
                            let new_grow_bounds = Bounds {
                                width: grow_bounds.width + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                x: shrink_bounds.x + amount,
                                width: shrink_bounds.width - amount,
                                ..shrink_bounds
                            };
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                        } else {
                            // Growing left
                            let new_grow_bounds = Bounds {
                                x: grow_bounds.x - amount,
                                width: grow_bounds.width + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                width: shrink_bounds.width - amount,
                                ..shrink_bounds
                            };
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                        }
                    }
                }
                Direction::Vertical => {
                    if shrink_bounds.height - amount < 100.0 {
                        return Ok(());
                    }

                    if is_shrinking {
                        if focused_idx < grow_idx {
                            // Focused is top, next grows up
                            let new_shrink_bounds = Bounds {
                                height: shrink_bounds.height - amount,
                                ..shrink_bounds
                            };
                            let new_grow_bounds = Bounds {
                                y: grow_bounds.y - amount,
                                height: grow_bounds.height + amount,
                                ..grow_bounds
                            };
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                        } else {
                            // Focused is bottom, previous grows down
                            let new_grow_bounds = Bounds {
                                height: grow_bounds.height + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                y: shrink_bounds.y + amount,
                                height: shrink_bounds.height - amount,
                                ..shrink_bounds
                            };
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                        }
                    } else {
                        if grow_idx < shrink_idx {
                            // Growing down
                            let new_grow_bounds = Bounds {
                                height: grow_bounds.height + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                y: shrink_bounds.y + amount,
                                height: shrink_bounds.height - amount,
                                ..shrink_bounds
                            };
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                        } else {
                            // Growing up
                            let new_grow_bounds = Bounds {
                                y: grow_bounds.y - amount,
                                height: grow_bounds.height + amount,
                                ..grow_bounds
                            };
                            let new_shrink_bounds = Bounds {
                                height: shrink_bounds.height - amount,
                                ..shrink_bounds
                            };
                            children[shrink_idx].update_bounds(new_shrink_bounds)?;
                            children[grow_idx].update_bounds(new_grow_bounds)?;
                        }
                    }
                }
            }

            Ok(())
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
