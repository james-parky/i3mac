use crate::container::Axis;
use crate::log::{Level, Log};
use core_graphics::{Direction, WindowId};

pub enum Message {
    LogicalNew,
    LogicalShiftFocus(Direction, WindowId),
    LogicalSplitRoot(Axis),
    LogicalSplitContainer(Axis, WindowId),
    LogicalSetFocused(WindowId),
    LogicalAddedWindow(WindowId),
    LogicalResizeWindow(WindowId, Direction),
}

impl Log for Message {
    fn level(&self) -> Level {
        use Message::*;

        match self {
            LogicalNew => Level::Info,
            LogicalShiftFocus(_, _) => Level::Trace,
            LogicalSplitRoot(_) => Level::Trace,
            LogicalSplitContainer(_, _) => Level::Trace,
            LogicalSetFocused(_) => Level::Trace,
            LogicalAddedWindow(_) => Level::Info,
            LogicalResizeWindow(_, _) => Level::Trace,
        }
    }

    fn message(&self) -> String {
        use Message::*;

        match self {
            LogicalNew => "logical display created".to_string(),
            LogicalShiftFocus(direction, window_id) => {
                format!("shifted focus {direction} to window {window_id}")
            }
            LogicalSplitRoot(axis) => {
                format!("splitting root container along {axis} axis")
            }
            LogicalSplitContainer(axis, window) => {
                format!("splitting container that owns window {window} along {axis} axis")
            }
            LogicalSetFocused(window) => format!("focused window {window}"),
            LogicalAddedWindow(window) => format!("added window {window}"),
            LogicalResizeWindow(window, direction) => {
                format!("resize window {window} in {direction}")
            }
        }
    }
}
