use crate::container::split::Split;
use crate::container::{Axis, Container, Window};
use core_graphics::Bounds;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub(super) struct Leaf {
    pub bounds: Bounds,
    pub padding: f64,
    pub window: Window,
}

impl Leaf {
    pub fn new(bounds: Bounds, padding: f64, window: Window) -> Self {
        Self {
            bounds,
            padding,
            window,
        }
    }

    pub fn split(&self, axis: Axis) -> Split {
        let outer_bounds = self.bounds.with_pad(-self.padding);
        let children = vec![Container::Leaf(*self)];

        Split::new(outer_bounds, axis, self.padding, children)
    }
}
