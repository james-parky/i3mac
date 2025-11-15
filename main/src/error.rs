#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Error {
    AxUi(ax_ui::Error),
    WindowNotFound,
    DisplayNotFound,
    CannotAddWindowToLeaf,
    CannotSplitEmptyContainer,
    CannotSplitAlreadySplitContainer,
    CannotFocusEmptyDisplay,
    CannotResizeRoot,
    CannotMoveWindowToSameDisplay,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
