use ax_ui::{Callback, Observer};
use core_foundation::CFRunLoopRun;
use core_graphics::{Bounds, CGPoint, CGSize};
use std::rc::Rc;

#[derive(Debug)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
enum Direction {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug, Clone)]
struct Window<'a> {
    cg: &'a core_graphics::Window,
    ax: ax_ui::Window,
    lock_observer: Observer,
    bounds: Bounds,
}

struct LockContext {
    window: Rc<ax_ui::Window>,
    point: CGPoint,
    size: CGSize,
}

impl<'a> Window<'a> {
    fn init(&mut self) -> Result<()> {
        self.ax
            .move_to(self.bounds.x, self.bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .resize(self.bounds.width, self.bounds.height)
            .map_err(Error::AxUi)
    }

    fn try_new(cg_window: &'a core_graphics::Window, bounds: Bounds) -> Result<Self> {
        let search_name = cg_window.name().unwrap().to_string();
        let mut ax_window =
            ax_ui::Window::new(cg_window.owner_pid(), search_name).map_err(Error::AxUi)?;

        let context = LockContext {
            window: Rc::new(ax_window),
            point: bounds.point(),
            size: bounds.size(),
        };

        let lock_callback = Rc::new(Callback::new(context, |ctx| {
            let _ = ctx.window.resize(ctx.size.width, ctx.size.height);
            let _ = ctx.window.move_to(ctx.point.x, ctx.point.y);
        }));

        let observer =
            Observer::try_new(cg_window.owner_pid(), &lock_callback).map_err(Error::AxUi)?;
        observer
            .add_notification(ax_window.window_ref(), "AXResized", lock_callback.ctx)
            .map_err(Error::AxUi)?;
        observer
            .add_notification(ax_window.window_ref(), "AXMoved", lock_callback.ctx)
            .map_err(Error::AxUi)?;

        observer.run();

        Ok(Self {
            cg: cg_window,
            ax: ax_window,
            lock_observer: observer,
            bounds,
        })
    }
}

#[derive(Debug)]
enum Container<'a> {
    Leaf(Window<'a>),
    Split {
        direction: Direction,
        children: Vec<Container<'a>>,
    },
}

#[derive(Debug)]
struct Display<'a> {
    id: u64,
    bounds: Bounds,
    root: Container<'a>,
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

impl<'a> Display<'a> {
    fn try_new(display: &'a core_graphics::Display) -> Result<Self> {
        let container = match display.windows.len() {
            1 => {
                let mut w = Window::try_new(&display.windows[0], display.bounds)?;
                w.init()?;
                Container::Leaf(w)
            }
            n => {
                let widths = split_n(display.bounds.width, n);
                let xs = xs_from_widths(display.bounds.x, &widths);
                let bounds: Vec<Bounds> = widths
                    .iter()
                    .zip(xs.iter())
                    .map(|(&width, &x)| Bounds {
                        width,
                        x,
                        ..display.bounds
                    })
                    .collect();

                let children = (0..n)
                    .map(|i| match Window::try_new(&display.windows[i], bounds[i]) {
                        Ok(mut window) => {
                            window.init()?;
                            Ok(window)
                        }
                        Err(e) => Err(e),
                    })
                    .collect::<Result<Vec<Window>>>()?;

                Container::Split {
                    direction: Direction::default(),
                    children: children.into_iter().map(Container::Leaf).collect(),
                }
            }
        };

        Ok(Self {
            // TODO: get id
            id: 0,
            bounds: display.bounds,
            root: container,
        })
    }
}

struct CloseContext<'a> {
    display: &'a Display<'a>,
}

fn main() {
    let _ = core_graphics::Display::all()
        .unwrap()
        .values()
        .map(Display::try_new)
        .collect::<Result<Vec<_>>>()
        .unwrap();

    unsafe { CFRunLoopRun() };
}
