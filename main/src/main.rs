mod display;
mod window;

use crate::display::Display;
use core_foundation::CFRunLoopRun;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
}

type Result<T> = std::result::Result<T, Error>;

fn main() {
    std::thread::spawn(|| unsafe { CFRunLoopRun() });

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        match core_graphics::Display::all() {
            Ok(display_map) => {
                let _displays: HashMap<_, _> = display_map
                    .iter()
                    .filter_map(|(id, display)| Display::try_new(display).ok().map(|d| (*id, d)))
                    .collect();
            }
            Err(err) => {
                println!("POLL ERROR: {err:?}")
            }
        }
    }
}
