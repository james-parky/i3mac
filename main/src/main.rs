use core_foundation::CFRunLoopRun;
use core_graphics::{CGPoint, CGRect, CGSize, DisplayId};

#[derive(Debug)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Debug)]
struct Window<'a> {
    cg: &'a core_graphics::Window,
    ax: &'a ax_ui::Window,
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
    let mut axw = ax_ui::Window::new(cgw.owner_pid(), cgw.bounds()).unwrap();
    // TODO: just store point and rect?
    let point = CGPoint {
        x: cgw.bounds().x,
        y: cgw.bounds().y,
    };
    let size = CGSize {
        width: cgw.bounds().width,
        height: cgw.bounds().height,
    };
    let e = axw.attach_lock_callback(point, size);
    println!("{e:?}");

    let w = Window { cg: cgw, ax: &axw };
    let left_display = Display {
        id: 3,
        bounds: d.bounds,
        root: Container::Leaf(w),
    };

    println!("{left_display:?}");

    unsafe { CFRunLoopRun() };
}
