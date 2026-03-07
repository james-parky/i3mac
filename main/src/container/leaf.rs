use crate::container::split::Split;
use crate::container::{Axis, Container, Window};
use core_graphics::{Bounds, WindowId};

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

    #[cfg(test)]
    pub(crate) fn dummy(window_id: &WindowId) -> Container {
        use crate::container::tests::dummy_bounds;
        Container::Leaf(Leaf::new(dummy_bounds(), 0.0, Window::dummy(*window_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_graphics::WindowId;

    // Bounds and padding should remain the same, and the new child should be
    // the leaf itself.
    #[test]
    fn test_split() {
        let bounds = Bounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let padding = 0.0;
        let window = Window {
            id: WindowId::from(0u64),
            min_width: 0.0,
            min_height: 0.0,
        };

        let leaf = Leaf::new(bounds, padding, window);
        let split = leaf.split(Axis::Vertical);

        assert_eq!(split.padding, padding);
        assert_eq!(split.bounds, bounds);
        assert!(split.children.len() == 1 && split.children[0] == Container::Leaf(leaf));
    }
}
