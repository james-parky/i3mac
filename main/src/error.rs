use crate::display;
use crate::display::logical;
use core_graphics::DisplayId;
use std::io;

#[derive(Debug)]
pub enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
    WindowNotFound,
    DisplayNotFound,
    CannotAddWindowToLeaf,
    CannotSplitEmptyContainer,
    CannotSplitAlreadySplitContainer,
    CannotFocusEmptyDisplay,
    CannotResizeRoot,
    CannotFindParentLeaf,
    ExpectedSplitContainer,
    CouldNotRemoveWindow,
    CannotFitWindow,
    PhysicalDoesNotContainLogical(DisplayId, display::physical::Id),
    CreateLogger(io::Error),
    LogicalAlreadyExists(logical::Id),
    NoAvailableLogical,
    NoDisplays,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
