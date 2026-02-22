use crate::display::LogicalDisplayId;
use core_graphics::DisplayId;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Error {
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
    PhysicalDoesNotContainLogical(DisplayId, LogicalDisplayId),
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
