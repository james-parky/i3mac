use crate::log::{Level, Prefix};
use crate::{
    container::{self, Axis, Container, Empty},
    display::log::Message::{
        LogicalAddedWindow, LogicalNew, LogicalResizeWindow, LogicalSetFocused, LogicalShiftFocus,
        LogicalSplitContainer, LogicalSplitRoot,
    },
    error::{Error, Result},
    log::{Log, Logger},
    status_bar::StatusBar,
};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

#[derive(PartialEq, Eq, Copy, Clone, Hash, Ord, PartialOrd)]
pub struct Id(pub usize);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "LD{}", self.0)
    }
}

impl From<usize> for Id {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl Id {
    fn as_log_prefix(&self) -> Prefix {
        Prefix::new(format!("LD{}", self.0))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Config {
    window_padding: Option<f64>,
    log_level: Level,
}

impl From<crate::display::physical::Config> for Config {
    fn from(config: crate::display::physical::Config) -> Self {
        Self {
            window_padding: config.window_padding,
            log_level: config.log_level,
        }
    }
}

impl Config {
    pub fn window_padding(&self) -> f64 {
        self.window_padding.unwrap_or_default()
    }
}

#[derive(Debug)]
pub(crate) struct Display<S> {
    root: Container,
    config: Config,
    logger: Logger,
    state: S,
}

pub struct NoWindows;
pub struct SomeWindows {
    focused_window: WindowId,
}

impl Display<NoWindows> {
    /// Create a new `Display` with the provided `Bounds` and `Config`.
    ///
    /// The height of a `Display` does **not** include the screen space reserved
    /// for the Apple menu bar at the top of the screen, and i3mac's status bar
    /// at the bottom of the screen.
    pub(crate) fn new(id: Id, cg_bounds: Bounds, config: Config) -> Self {
        // Core Graphics bounds -- the bounds used for a `physical::Display` do
        // not include the Apple menu bar so we need to subtract it to get the
        // usable area for windows to exist in.
        const MENU_BAR_HEIGHT: f64 = 37.0;

        // We also subtract the height of the i3-style `StatusBar` added to the
        // bottom of the screen.
        let bounds = Bounds {
            height: cg_bounds.height - MENU_BAR_HEIGHT - StatusBar::HEIGHT,
            y: cg_bounds.y + MENU_BAR_HEIGHT,
            ..cg_bounds
        };

        let mut logger =
            Logger::try_new("/dev/stdout", config.log_level, id.as_log_prefix()).unwrap();

        LogicalNew.log(&mut logger);
        Display {
            root: Container::Empty(Empty::new(bounds)),
            config,
            logger,
            state: NoWindows,
        }
    }

    pub fn add_window(self, window: container::Window) -> Result<Display<SomeWindows>> {
        let mut container = self.root;
        container.add_window(window, self.config.window_padding())?;

        let ret = Display::<SomeWindows> {
            root: container,
            config: self.config,
            logger: self.logger,
            state: SomeWindows {
                focused_window: window.id,
            },
        };

        Ok(ret)
    }

    pub fn split(&mut self, axis: Axis) -> Result<()> {
        self.root.split(axis)
    }
}

impl Display<SomeWindows> {
    /// Return's the logical display's currently focussed window's ID.
    pub fn focused_window(&self) -> WindowId {
        self.state.focused_window
    }

    /// Shift focus within the logical display's managed windows in some
    /// direction. If the focus cannot shift any more in the provided
    /// `direction`, focus remains on the currently focused window.
    // In order to switch focus in some direction:
    //  - If there are no windows at all (including the one that should be
    //    currently focused), return some error.
    //  - Create a list of all window, sorted by either x or y position, based
    //    on the given direction.
    //  - Find the focused window in this list.
    //  - If there are no more windows in the direction of the shift, return.
    //  - If there is one, focus it and return.
    pub fn shift_focus(&mut self, direction: Direction) -> Result<WindowId> {
        use Direction::*;

        // Declared with a block to drop mutability after sorting.
        let windows: Vec<_> = {
            let mut windows: Vec<_> = self.window_bounds().into_iter().collect();
            if windows.is_empty() {
                return Err(Error::CannotFocusEmptyDisplay);
            }

            match direction {
                Left | Right => windows.sort_by(|a, b| a.1.x.total_cmp(&b.1.x)),
                Up | Down => windows.sort_by(|a, b| a.1.y.total_cmp(&b.1.y)),
            };

            windows
        };

        let current_focussed_index = windows
            .iter()
            .position(|(id, _)| *id == self.state.focused_window)
            .unwrap_or(0);

        let next_focus = match (current_focussed_index, direction) {
            (n, Left | Up) if n != 0 => windows[n - 1].0,
            (n, Right | Down) if n < windows.len() - 1 => windows[n + 1].0,
            _ => windows[current_focussed_index].0,
        };

        self.state.focused_window = next_focus;

        LogicalShiftFocus(direction, next_focus).log(&mut self.logger);
        Ok(next_focus)
    }

    /// Returns a map of window ID to its bounds for all windows the logical
    /// display manages.
    pub fn window_bounds(&self) -> HashMap<WindowId, Bounds> {
        self.root.window_bounds_by_id()
    }

    /// Returns the set of all window IDs the logical display manages.
    pub(crate) fn window_ids(&self) -> HashSet<WindowId> {
        self.root.window_ids()
    }

    /// Split the logical display's focused window's container along the
    /// provided `axis`.
    pub fn split(&mut self, axis: Axis) -> Result<()> {
        // Safety: If we are a Display::<SomeWindows> then there is guaranteed
        //         to be a focused window and that windows is guaranteed to have
        //         a parent.
        let container = self
            .root
            .parent_leaf_of_window_mut(self.state.focused_window)
            .unwrap();

        LogicalSplitContainer(axis, self.state.focused_window).log(&mut self.logger);
        container.split(axis)
    }

    /// Set the logical display's focussed window to `window_id` or return an
    /// error if the logical display does not manage the window.
    pub fn set_focused_window(&mut self, window_id: WindowId) -> Result<()> {
        if self.window_ids().contains(&window_id) {
            self.state.focused_window = window_id;
            LogicalSetFocused(window_id).log(&mut self.logger);
            Ok(())
        } else {
            Err(Error::WindowNotFound)
        }
    }

    pub fn remove_window(self, window_id: WindowId) -> Result<RemoveResult> {
        let mut root = self.root;

        match root.remove_window(window_id, self.config.window_padding())? {
            container::RemoveResult::NotFound => Err(Error::WindowNotFound),
            container::RemoveResult::BecomeEmpty => Ok(RemoveResult::NowEmpty(Display {
                root,
                config: self.config,
                logger: self.logger,
                state: NoWindows,
            })),
            container::RemoveResult::Removed => {
                let new_focused = if self.state.focused_window == window_id {
                    // Safety: since the remove result was not BecomeEmpty, we
                    //         know there is at least one more window to become
                    //         the newly focused one.
                    root.window_ids().iter().next().copied().unwrap()
                } else {
                    self.state.focused_window
                };

                Ok(RemoveResult::StillHasWindows(Display {
                    root,
                    config: self.config,
                    logger: self.logger,
                    state: SomeWindows {
                        focused_window: new_focused,
                    },
                }))
            }
        }
    }

    /// Add a window to the logical display, accounting for its configured
    /// minimum bounds.
    ///
    /// The window will be added as a sibling of the currently focused window if
    /// one exists, otherwise it will be added to the root.
    // When adding a window to a logical display, see if there is a previously
    // focused window.
    // If so:
    //  - Find the split that owns the window
    //  - Add new window as a child to that split
    // If there is no window:
    //  - Add new window as a child of the root (horizontal split)
    pub fn add_window(&mut self, window: container::Window) -> Result<()> {
        // Safety: If we are a Display::<SomeWindows> then there is guaranteed
        //         to be a focused window and that windows is guaranteed to have
        //         a parent.
        let container = self
            .root
            .get_parent_of_window_mut(self.state.focused_window)
            .unwrap();

        container.add_window(window, self.config.window_padding())?;
        LogicalAddedWindow(window.id).log(&mut self.logger);

        self.state.focused_window = window.id;
        LogicalSetFocused(window.id).log(&mut self.logger);

        Ok(())
    }

    /// If there is a focussed window, resize it in `direction` by the
    /// configured amount, accounting for any padding.
    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        self.resize_window_in_direction(self.state.focused_window, direction)?;
        LogicalResizeWindow(self.state.focused_window, direction).log(&mut self.logger);

        Ok(())
    }

    /// Resize the window corresponding to `id` in `direction` by the configured
    /// amount, accounting for any padding.
    pub fn resize_window_in_direction(&mut self, id: WindowId, direction: Direction) -> Result<()> {
        let padding = self.config.window_padding();
        self.root.resize_window(id, direction, padding)?;
        LogicalResizeWindow(id, direction).log(&mut self.logger);
        Ok(())
    }
}

pub enum RemoveResult {
    StillHasWindows(Display<SomeWindows>),
    NowEmpty(Display<NoWindows>),
}
