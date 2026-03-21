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
pub(crate) struct Display {
    id: Id,
    root: Container,
    focused_window: Option<WindowId>,
    config: Config,
    logger: Logger,
}

impl Display {
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
            id,
            root: Container::Empty(Empty::new(bounds)),
            focused_window: None,
            config,
            logger,
        }
    }

    /// Return's the logical display's currently focussed window's ID.
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
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

        // TODO: should really return an error here for both unwraps since the
        //       focused window SHOULD exist
        let current_focused = self.focused_window.unwrap_or(windows[0].0);
        let current_focussed_index = windows
            .iter()
            .position(|(id, _)| *id == current_focused)
            .unwrap_or(0);

        let next_focus = match (current_focussed_index, direction) {
            (n, Left | Up) if n != 0 => windows[n - 1].0,
            (n, Right | Down) if n < windows.len() - 1 => windows[n + 1].0,
            _ => windows[current_focussed_index].0,
        };

        self.focused_window = Some(next_focus);

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
    ///
    /// If there is no focussed window, change the current split direction of
    /// the logical display's `root` container.
    pub fn split(&mut self, axis: Axis) -> Result<()> {
        let container = if let Some(id) = self.focused_window {
            let c = self
                .root
                .parent_leaf_of_window_mut(id)
                .ok_or(Error::CannotFindParentLeaf)?;

            LogicalSplitContainer(axis, id).log(&mut self.logger);
            c
        } else {
            LogicalSplitRoot(axis).log(&mut self.logger);
            &mut self.root
        };

        container.split(axis)
    }

    /// Set the logical display's focussed window to `window_id` or return an
    /// error if the logical display does not manage the window.
    pub fn set_focused_window(&mut self, window_id: WindowId) -> Result<()> {
        if self.window_ids().contains(&window_id) {
            self.focused_window = Some(window_id);
            LogicalSetFocused(window_id).log(&mut self.logger);
            Ok(())
        } else {
            Err(Error::WindowNotFound)
        }
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<Option<WindowId>> {
        let padding = self.config.window_padding();
        let removed = self.root.remove_window(window_id, padding)?;

        if self.focused_window == Some(window_id) {
            self.focused_window = self.window_ids().iter().next().copied();
            LogicalSetFocused(window_id).log(&mut self.logger);
        }

        Ok(removed)
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
        // TODO: should probably error here if there is a focused window but
        //       there is no parent for it
        let container = match self.focused_window {
            None => &mut self.root,
            Some(id) => match self.root.get_parent_of_window_mut(id) {
                Some(c) => c,
                None => &mut self.root,
            },
        };

        container.add_window(window, self.config.window_padding())?;
        LogicalAddedWindow(window.id).log(&mut self.logger);

        self.focused_window = Some(window.id);
        LogicalSetFocused(window.id).log(&mut self.logger);

        Ok(())
    }

    /// If there is a focussed window, resize it in `direction` by the
    /// configured amount, accounting for any padding.
    pub fn resize_focused_window(&mut self, direction: Direction) -> Result<()> {
        if let Some(focused_id) = self.focused_window {
            self.resize_window_in_direction(focused_id, direction)?;
            LogicalResizeWindow(focused_id, direction).log(&mut self.logger);
        }

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
