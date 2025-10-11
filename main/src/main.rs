use core_foundation::CFRunLoopRun;
use core_graphics::{Bounds, CGPoint, CGRect, CGSize, DisplayId};

#[derive(Debug)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Debug)]
struct Window<'a> {
    cg: &'a core_graphics::Window,
    ax: ax_ui::Window,
}

impl<'a> Window<'a> {
    fn try_new(cg_window: &'a core_graphics::Window, bounds: Bounds) -> Result<Self> {
        let mut ax_window =
            ax_ui::Window::new(cg_window.owner_pid(), cg_window.bounds()).map_err(Error::AxUi)?;
        ax_window
            .attach_lock_callback(bounds.point(), bounds.size())
            .map_err(Error::AxUi)?;

        ax_window.move_to(bounds.x, bounds.y).map_err(Error::AxUi)?;
        ax_window
            .resize(bounds.width, bounds.height)
            .map_err(Error::AxUi)?;

        Ok(Self {
            cg: cg_window,
            ax: ax_window,
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

fn main() {
    let displays = core_graphics::Display::all().unwrap();
    let d = displays.get(&3usize.into()).unwrap();
    let cgw = &d.windows[0];

    let w = Window::try_new(&cgw, d.bounds.with_pad(40.0)).unwrap();
    let left_display = Display {
        id: 3,
        bounds: d.bounds,
        root: Container::Leaf(w),
    };

    println!("{left_display:?}");

    unsafe { CFRunLoopRun() };
}
