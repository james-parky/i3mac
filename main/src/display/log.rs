use crate::{
    container::Axis,
    display::logical,
    log::{Level, Log},
};
use core_graphics::{Direction, WindowId};

pub enum Message {
    LogicalNew,
    LogicalShiftFocus(Direction, WindowId),
    LogicalSplitRoot(Axis),
    LogicalSplitContainer(Axis, WindowId),
    LogicalSetFocused(WindowId),
    LogicalAddedWindow(WindowId),
    LogicalResizeWindow(WindowId, Direction),

    PhysicalNew,
    PhysicalShiftFocus(Direction, WindowId),
    PhysicalSplit(Axis),
    PhysicalSetFocused(WindowId),
    PhysicalAddedWindow(WindowId),
    PhysicalAddedWindowToLogical(WindowId, logical::Id),
    PhysicalResizeWindow(WindowId, Direction),
    PhysicalRemovedWindow(WindowId),
    PhysicalAddedLogical(logical::Id),
    PhysicalRemovedLogical(logical::Id),
    PhysicalSwitchActive(logical::Id),
    PhysicalResizeFocused(Direction),
    PhysicalSwitchDisplay(logical::Id),
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

            PhysicalNew => Level::Info,
            PhysicalShiftFocus(_, _) => Level::Trace,
            PhysicalSplit(_) => Level::Trace,
            PhysicalSetFocused(_) => Level::Trace,
            PhysicalAddedWindow(_) => Level::Info,
            PhysicalAddedWindowToLogical(_, _) => Level::Info,
            PhysicalResizeWindow(_, _) => Level::Trace,
            PhysicalRemovedWindow(_) => Level::Info,
            PhysicalAddedLogical(_) => Level::Info,
            PhysicalRemovedLogical(_) => Level::Info,
            PhysicalSwitchActive(_) => Level::Trace,
            PhysicalResizeFocused(_) => Level::Trace,
            PhysicalSwitchDisplay(_) => Level::Trace,
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

            PhysicalNew => "physical display created".to_string(),
            PhysicalShiftFocus(direction, window_id) => {
                format!("shifted focus {direction} to window {window_id}")
            }
            PhysicalSplit(axis) => format!("splitting container along {axis} axis"),
            PhysicalSetFocused(window) => format!("focused window {window}"),
            PhysicalAddedWindow(window) => format!("added window {window}"),
            PhysicalAddedWindowToLogical(window, logical) => {
                format!("added window {window} to {logical:?}")
            }
            PhysicalResizeWindow(window, direction) => {
                format!("resize window {window} in {direction}")
            }
            PhysicalRemovedWindow(window) => format!("removed window {window}"),
            PhysicalAddedLogical(logical) => format!("added logical display {logical}"),
            PhysicalRemovedLogical(logical) => format!("removed logical display {logical}"),
            PhysicalSwitchActive(active) => format!("switching active display {active}"),
            PhysicalResizeFocused(direction) => format!("resized focused window {direction}"),
            PhysicalSwitchDisplay(display) => format!("switching to {display:?}"),
        }
    }
}
