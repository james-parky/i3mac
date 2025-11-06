mod display;
mod window;

use crate::display::Display;
use core_foundation::{CFRunLoopRun, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::DisplayId;
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Eq, PartialEq)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
    CGWindowMissingName(String),
}

macro_rules! log_or_continue {
    ($func:expr, $msg:literal) => {
        match $func {
            Ok(x) => x,
            Err(err) => {
                println!("{}: {err:?}", $msg);
                continue;
            }
        }
    };
}
type Result<T> = std::result::Result<T, Error>;

fn main() {
    std::thread::spawn(|| unsafe {
        CFRunLoopRun();
    });

    let mut prev_displays: HashMap<DisplayId, Display> = HashMap::new();

    loop {
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) }

        match core_graphics::Display::all() {
            Ok(cg_displays_map) => {
                for (display_id, cg_display) in cg_displays_map {
                    match prev_displays.get(&display_id) {
                        Some(display) if display.window_ids() == cg_display.window_ids() => {
                            continue;
                        }
                        _ => {}
                    }
                    record_display(&mut prev_displays, display_id, cg_display);
                }
            }
            Err(err) => {
                println!("POLL ERROR: {err:?}")
            }
        }
    }
}

fn record_display(
    displays: &mut HashMap<DisplayId, Display>,
    display_id: DisplayId,
    display: core_graphics::Display,
) {
    let new_display = match Display::try_new(display) {
        Ok(display) => display,
        Err(err) => {
            println!("failed to record new display: {err:?}");
            return;
        }
    };
    displays.insert(display_id, new_display);
}
