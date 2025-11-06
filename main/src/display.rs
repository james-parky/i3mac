use crate::{Error, Result, window::Window};
use core_graphics::Bounds;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Debug, Default, Clone, Hash)]
enum Direction {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug)]
enum Container {
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
    fn cg_windows(&self) -> HashSet<&core_graphics::Window> {
        match &self {
            Self::Leaf { window, .. } => {
                let mut ret: HashSet<&core_graphics::Window> = HashSet::with_capacity(1);
                ret.insert(window.cg());
                ret
            }
            Self::Split { children, .. } => {
                let mut ret: HashSet<&core_graphics::Window> = HashSet::new();
                for child in children {
                    ret.extend(child.cg_windows());
                }
                ret
            }
        }
    }

    fn window_ids(&self) -> HashSet<u64> {
        match self {
            Self::Leaf { window, .. } => {
                let mut ret: HashSet<u64> = HashSet::with_capacity(1);
                ret.insert(window.cg().number());
                ret
            }
            Self::Split { children, .. } => children
                .iter()
                .flat_map(|child| child.window_ids())
                .collect(),
        }
    }

    fn try_from_window(window: core_graphics::Window, bounds: Bounds) -> Result<Self> {
        let mut window = Window::try_new(window, bounds)?;
        window.init()?;
        Ok(Container::Leaf { bounds, window })
    }

    fn try_from_windows(windows: HashSet<core_graphics::Window>, bounds: Bounds) -> Result<Self> {
        // TODO: error when empty
        let n = windows.len();

        if n == 1 {
            let mut window = Window::try_new(windows.into_iter().nth(0).unwrap(), bounds)?;
            window.init()?;
            return Ok(Container::Leaf { bounds, window });
        }

        let mut minimum_widths: Vec<(core_graphics::Window, f64)> = Vec::with_capacity(n);
        for cg_window in windows {
            let ax_window = ax_ui::Window::new(cg_window.owner_pid(), cg_window.number().into())
                .map_err(Error::AxUi)?;
            // TODO: reasonable default?
            let min_width = ax_window
                .min_size()
                .map(|size| size.width)
                .unwrap_or(bounds.width / n as f64);
            minimum_widths.push((cg_window, min_width));
        }

        let widths = split_n_with_minimums(bounds.width, &minimum_widths);
        let xs = xs_from_widths(bounds.x, &widths);

        let mut children = Vec::with_capacity(n);

        for ((window, _), (&width, &x)) in
            minimum_widths.into_iter().zip(widths.iter().zip(xs.iter()))
        {
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
pub(crate) struct Display {
    bounds: Bounds,
    root: Container,
}

impl Display {
    pub(crate) fn try_new(display: core_graphics::Display) -> Result<Self> {
        let container = Container::try_from_windows(display.windows, display.bounds)?;

        Ok(Self {
            bounds: display.bounds,
            root: container,
        })
    }

    pub(crate) fn window_ids(&self) -> HashSet<u64> {
        self.root.window_ids()
    }

    pub(crate) fn cg_windows(&self) -> HashSet<&core_graphics::Window> {
        self.root.cg_windows()
    }
}

fn split_n_with_minimums(total: f64, windows: &[(core_graphics::Window, f64)]) -> Vec<f64> {
    let n = windows.len();
    let min_widths: Vec<f64> = windows.iter().map(|(_, min)| *min).collect();
    let total_min: f64 = min_widths.iter().sum();

    if total_min >= total {
        return min_widths;
    }

    let remaining = total - total_min;
    let extra_per_window = remaining / n as f64;
    min_widths
        .iter()
        .map(|min| min + extra_per_window)
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
