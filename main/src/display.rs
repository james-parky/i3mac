use crate::window::Window;
use crate::{Error, Result};
use ax_ui::{Callback, Observer};
use core_graphics::Bounds;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Default, Clone)]
enum Direction {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug, Clone)]
enum Container<'a> {
    Leaf {
        bounds: Bounds,
        window: Window<'a>,
    },
    Split {
        bounds: Bounds,
        direction: Direction,
        children: Vec<Container<'a>>,
    },
}

struct CloseContext<'a> {
    container: Rc<RefCell<Container<'a>>>,
    window: Window<'a>,
}

impl<'a> Container<'a> {
    fn try_from_window(window: &'a core_graphics::Window, bounds: Bounds) -> Result<Self> {
        let mut window = Window::try_new(window, bounds)?;
        window.init()?;
        Ok(Container::Leaf { bounds, window })
    }

    fn try_from_windows(windows: &'a [core_graphics::Window], bounds: Bounds) -> Result<Self> {
        // TODO: error when empty
        let n = windows.len();

        let widths = split_n(bounds.width, n);
        let xs = xs_from_widths(bounds.x, &widths);

        let mut children = Vec::with_capacity(n);

        for (window, (&width, &x)) in windows.iter().zip(widths.iter().zip(xs.iter())) {
            let child_bounds = Bounds { width, x, ..bounds };

            let child = Container::try_from_window(window, child_bounds)?;
            children.push(child);
        }

        let container = Container::Split {
            bounds,
            direction: Direction::default(),
            children,
        };

        Ok(container)
    }
}

#[derive(Debug)]
pub(crate) struct Display<'a> {
    id: u64,
    bounds: Bounds,
    root: Container<'a>,
}

impl<'a> Display<'a> {
    pub(crate) fn try_new(display: &'a core_graphics::Display) -> Result<Self> {
        let container = match display.windows.len() {
            1 => Container::try_from_window(&display.windows[0], display.bounds)?,
            _ => Container::try_from_windows(&display.windows, display.bounds)?,
        };

        Ok(Self {
            // TODO: get id
            id: 0,
            bounds: display.bounds,
            root: container,
        })
    }
}

fn split_n(total: f64, n: usize) -> Vec<f64> {
    let base = total / n as f64;
    let remainder = total % n as f64;

    (0..n)
        .map(|i| {
            if (i as f64) < remainder {
                base + 1.0
            } else {
                base
            }
        })
        .collect()
}

fn xs_from_widths(start: f64, widths: &[f64]) -> Vec<f64> {
    let mut xs = Vec::with_capacity(widths.len());
    xs.push(start);
    for w in widths.iter().skip(1) {
        xs.push(xs[xs.len() - 1] + w);
    }

    xs
}
