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
        children: Vec<Rc<RefCell<Container<'a>>>>,
    },
}

struct CloseContext<'a> {
    container: Rc<RefCell<Container<'a>>>,
    window: Window<'a>,
}

impl<'a> Container<'a> {
    fn try_from_window(
        window: &'a core_graphics::Window,
        bounds: Bounds,
    ) -> Result<Rc<RefCell<Self>>> {
        let mut window = Window::try_new(window, bounds)?;
        window.init()?;
        let container = Container::Leaf { bounds, window };
        Ok(Rc::new(RefCell::new(container)))
    }

    fn try_from_windows(
        windows: &'a [core_graphics::Window],
        bounds: Bounds,
    ) -> Result<Rc<RefCell<Self>>> {
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

        Ok(Rc::new(RefCell::new(container)))
    }

    fn attach_destroy_observer(
        container: Rc<RefCell<Container<'a>>>,
        root: Rc<RefCell<Container<'a>>>,
    ) -> Result<()> {
        // let container_clone = Rc::clone(&container);

        match &*container.borrow() {
            Container::Leaf { window, .. } => {
                let context = CloseContext {
                    container: Rc::clone(&root),
                    window: window.clone(),
                };

                let callback = Rc::new(Callback::new(context, |ctx| {
                    let mut root = ctx.container.borrow_mut();
                    let _ = root.remove_destroyed(ctx.window.ax());
                    println!("WINDOW DESTROYED");
                    root.recalc();
                }));

                let observer =
                    Observer::try_new(window.cg().owner_pid(), &callback).map_err(Error::AxUi)?;
                observer
                    .add_notification(
                        window.ax().window_ref(),
                        "AXUIElementDestroyed",
                        callback.ctx,
                    )
                    .map_err(Error::AxUi)?;

                observer.run();
                Ok(())
            }
            Container::Split { children, .. } => {
                for child in children.iter().cloned() {
                    Container::attach_destroy_observer(child, Rc::clone(&root))?;
                }
                Ok(())
            }
        }
    }

    fn recalc(&mut self) -> Result<()> {
        println!("Recalculating");
        if let Container::Split {
            bounds, children, ..
        } = self
        {
            println!("Split");
            let n = children.len();
            if n == 0 {
                return Ok(());
            }
            // TODO: for now assuming all container are horizontal
            let widths = split_n(bounds.width, n);
            let xs = xs_from_widths(bounds.x, &widths);
            for (child, (&width, &x)) in children.iter().zip(widths.iter().zip(xs.iter())) {
                let child_bounds = Bounds {
                    width,
                    x,
                    ..*bounds
                };
                println!("current bounds: {:?}, going to {:?}", bounds, child_bounds);

                child.borrow_mut().update_bounds(child_bounds)?;
            }
        }

        Ok(())
    }

    fn update_bounds(&mut self, bounds: Bounds) -> Result<()> {
        match self {
            Container::Leaf { window, bounds: b } => {
                *b = bounds;
                window.update_lock(bounds)
            }
            Container::Split { bounds: b, .. } => {
                *b = bounds;
                self.recalc();
                Ok(())
            }
        }
    }

    fn remove_destroyed(&mut self, destroyed: &ax_ui::Window) -> Result<bool> {
        match self {
            Container::Leaf { window, .. } => {
                Ok(window.ax().window_ref() == destroyed.window_ref())
            }
            Container::Split {
                children, bounds, ..
            } => {
                children
                    .retain(|child| child.borrow_mut().remove_destroyed(destroyed) == Ok(false));
                if children.is_empty() {
                    return Ok(true);
                }
                let parent_bounds = *bounds;
                if children.len() == 1 {
                    let inner = children.remove(0);
                    *self = inner.borrow().clone();
                    self.update_bounds(parent_bounds)?;
                }

                Ok(false)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Display<'a> {
    id: u64,
    bounds: Bounds,
    root: Rc<RefCell<Container<'a>>>,
}

impl<'a> Display<'a> {
    pub(crate) fn try_new(display: &'a core_graphics::Display) -> crate::Result<Self> {
        let container = match display.windows.len() {
            1 => Container::try_from_window(&display.windows[0], display.bounds)?,
            _ => Container::try_from_windows(&display.windows, display.bounds)?,
        };

        Container::attach_destroy_observer(Rc::clone(&container), Rc::clone(&container))?;

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
