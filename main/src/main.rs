mod display;
mod window;

use crate::display::Display;
use core_foundation::CFRunLoopRun;

#[derive(Debug, Eq, PartialEq)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
}

type Result<T> = std::result::Result<T, Error>;

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
