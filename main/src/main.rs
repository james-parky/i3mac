use ax_ui::Observer;
use core_foundation::CFRunLoopRun;
use core_graphics::{Bounds, CGPoint, CGRect, CGSize, DisplayId};

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

#[derive(Debug, Copy, Clone)]
struct Window<'a> {
    cg: &'a core_graphics::Window,
    ax: ax_ui::Window,
    lock_observer: Observer,
}

impl<'a> Window<'a> {
    fn try_new(cg_window: &'a core_graphics::Window, bounds: Bounds) -> Result<Self> {
        let mut ax_window =
            ax_ui::Window::new(cg_window.owner_pid(), cg_window.bounds()).map_err(Error::AxUi)?;

        ax_window.move_to(bounds.x, bounds.y).map_err(Error::AxUi)?;
        ax_window
            .resize(bounds.width, bounds.height)
            .map_err(Error::AxUi)?;

        let win_ref = ax_window.application_ref();

        let (lock_callback, ctx) = ax_window.create_lock_callback(bounds.point(), bounds.size());
        let observer =
            Observer::try_new(cg_window.owner_pid(), lock_callback).map_err(Error::AxUi)?;
        observer
            .add_notification(win_ref, "AXResized", ctx)
            .map_err(Error::AxUi)?;
        observer
            .add_notification(win_ref, "AXMoved", ctx)
            .map_err(Error::AxUi)?;

        Ok(Self {
            cg: cg_window,
            ax: ax_window,
            lock_observer: observer,
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
    bounds: core_graphics::Bounds,
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
    fn try_new(cg_display: &'a core_graphics::Display) -> Result<Self> {
        let container = match cg_display.windows.len() {
            1 => Container::Leaf(Window::try_new(&cg_display.windows[0], cg_display.bounds)?),
            n => {
                let widths = split_n(cg_display.bounds.width, n);
                let xs = xs_from_widths(cg_display.bounds.x, &widths);

                Container::Split {
                    direction: Direction::default(),
                    children: cg_display
                        .windows
                        .iter()
                        .enumerate()
                        .map(|(i, cgw)| {
                            Window::try_new(
                                cgw,
                                Bounds {
                                    width: widths[i],
                                    x: xs[i],
                                    height: cg_display.bounds.height,
                                    y: cg_display.bounds.y,
                                },
                            )
                        })
                        .collect::<Result<Vec<_>>>()?
                        .iter()
                        .map(|w| Container::Leaf(*w))
                        .collect(),
                }
            }
        };

        Ok(Self {
            // TODO: get id
            id: 0,
            bounds: cg_display.bounds,
            root: container,
        })
    }
}

fn main() {
    let displays = core_graphics::Display::all()
        .unwrap()
        .values()
        .map(Display::try_new)
        .collect::<Result<Vec<_>>>()
        .unwrap();

    // let cgw = &d.windows[0];
    //
    // let w = Window::try_new(&cgw, d.bounds.with_pad(40.0)).unwrap();
    // let left_display = Display {
    //     id: 3,
    //     bounds: d.bounds,
    //     root: Container::Leaf(w),
    // };
    //
    // println!("{left_display:?}");

    unsafe { CFRunLoopRun() };
}
