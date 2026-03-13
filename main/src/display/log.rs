use crate::display::physical;
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

    FocusLogical(logical::Id, WindowId),
    Split(Axis),
    SetActivePhysical(physical::Id),
    SwitchToLogical(physical::Id, logical::Id),
    RemovedEmptyLogical(logical::Id),
    ChoseNewLogicalId(logical::Id),
    NoNewLogicalIds,
    AddPhysical(physical::Id, logical::Id),
    AddLogical(physical::Id, logical::Id),
    AddingWindow(WindowId, physical::Id),
    CouldNotFitWindow(WindowId, logical::Id),
    AddedWindow(WindowId, logical::Id),
    RemovedWindow(WindowId, physical::Id),
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

            FocusLogical(_, _) => Level::Info,
            Split(_) => Level::Info,
            SetActivePhysical(_) => Level::Info,
            SwitchToLogical(_, _) => Level::Info,
            RemovedEmptyLogical(_) => Level::Info,
            ChoseNewLogicalId(_) => Level::Trace,
            NoNewLogicalIds => Level::Trace,
            AddPhysical(_, _) => Level::Info,
            AddLogical(_, _) => Level::Info,
            AddingWindow(_, _) => Level::Info,
            CouldNotFitWindow(_, _) => Level::Info,
            AddedWindow(_, _) => Level::Info,
            RemovedWindow(_, _) => Level::Info,
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

            FocusLogical(logical, window) => {
                format!("focus window {window} on logical display {logical:?}")
            }
            Split(axis) => format!("split focused container along {axis:?}"),
            SetActivePhysical(physical) => format!("set display {physical} active"),
            SwitchToLogical(physical, logical) => format!("switching to {logical:?} on {physical}"),
            RemovedEmptyLogical(logical) => format!("removed empty logical display {logical:?}"),
            ChoseNewLogicalId(logical) => format!("new logical display will be {logical:?}"),
            NoNewLogicalIds => "no new logical IDs left to create display".to_string(),
            AddPhysical(physical, logical) => {
                format!("added physical display {physical} with {logical:?}")
            }
            AddLogical(physical, logical) => {
                format!("added logical display {logical:?} to {physical}")
            }
            AddingWindow(window, physical) => format!("adding window {window} to {physical}"),
            CouldNotFitWindow(window, logical) => {
                format!("could not fit window {window} on {logical:?}")
            }
            AddedWindow(window, logical) => format!("added window {window} to {logical:?}"),
            RemovedWindow(window, physical) => format!("removed window {window} from {physical}"),
        }
    }
}
