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

                // Calculate new sizes based on current split direction
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

                // Update existing children
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

                    child.update_bounds(new_bounds)?;
                }

                // Add new child
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
                if let Some(i) = children.iter().position(|child| matches!(child, Self::Leaf{window,..} if window.cg().number()==window_id)){
                    children.remove(i);
                    if children.is_empty() {
                        *self = Self::Empty {bounds: saved_bounds};
                    } else {
                        self.recalculate_layout(saved_bounds)?;
                    }

                    return Ok(true);
                }
                for child in children.iter_mut() {
                    if child.remove_window(window_id)? {
                        self.recalculate_layout(saved_bounds)?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
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

    fn recalculate_layout(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        match self {
            Self::Empty { .. } => Ok(()),
            Self::Leaf { bounds, window, .. } => {
                *bounds = new_bounds;
                window.update_bounds(new_bounds)
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
                            child.recalculate_layout(child_bounds)?;
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
                            child.recalculate_layout(child_bounds)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn update_bounds(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        self.recalculate_layout(new_bounds)
    }
}
