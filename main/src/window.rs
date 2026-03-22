use crate::{error::Error, error::Result};
use core_graphics::Bounds;
use std::hash::Hash;

#[derive(Debug)]
pub(crate) struct Window {
    cg: core_graphics::Window,
    ax: ax_ui::Window,
    bounds: Bounds,
    /// If true, the window been toggled floating by the user. These windows are
    /// kept track of, but not managed, and therefore not included in container
    /// bounds calculations.
    is_floating: bool,
    /// If true, the window been minimised by the window manager but is still
    /// under management. When a window is minimised, either through
    /// user interaction, or the AXUI API, Core Graphics stops reporting its
    /// window ID. This causes issues with the minimisation/un-minimisation
    /// process performed during logical display focus shift; so keep track.
    is_minimised: bool,
}

impl Hash for Window {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cg.number().hash(state);
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.cg.number() == other.cg.number()
    }
}

impl Eq for Window {}

impl TryFrom<core_graphics::Window> for Window {
    type Error = Error;

    fn try_from(value: core_graphics::Window) -> std::result::Result<Self, Self::Error> {
        let ax_window =
            ax_ui::Window::new(value.owner_pid(), value.number()).map_err(Error::AxUi)?;
        ax_window.unminimise().map_err(Error::AxUi)?;
        ax_window.try_focus().map_err(Error::AxUi)?;

        Ok(Self {
            bounds: value.bounds().clone(),
            cg: value,
            ax: ax_window,
            is_floating: false,
            is_minimised: false,
        })
    }
}
impl Window {
    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub(crate) fn ax(&self) -> &ax_ui::Window {
        &self.ax
    }

    pub(crate) fn cg(&self) -> &core_graphics::Window {
        &self.cg
    }

    pub(crate) fn is_floating(&self) -> bool {
        self.is_floating
    }

    pub(crate) fn is_minimised(&self) -> bool {
        self.is_minimised
    }

    pub(crate) fn set_floating(&mut self, is_floating: bool) {
        self.is_floating = is_floating;
    }

    pub(crate) fn unminimise(&mut self) -> Result<()> {
        self.ax.unminimise().map_err(Error::AxUi)?;
        self.is_minimised = false;

        Ok(())
    }

    pub(crate) fn minimise(&mut self) -> Result<()> {
        self.ax.minimise().map_err(Error::AxUi)?;
        self.is_minimised = true;

        Ok(())
    }

    pub(crate) fn init(&mut self) -> Result<()> {
        self.ax
            .try_move_to(self.bounds.x, self.bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .try_resize(self.bounds.width, self.bounds.height)
            .map_err(Error::AxUi)?;

        Ok(())
    }

    pub fn update_bounds(&mut self, new_bounds: Bounds) -> Result<()> {
        self.bounds = new_bounds;

        self.ax
            .try_move_to(new_bounds.x, new_bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .try_resize(new_bounds.width, new_bounds.height)
            .map_err(Error::AxUi)?;

        Ok(())
    }
}
