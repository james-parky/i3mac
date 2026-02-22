use crate::{error::Error, error::Result};
use core_graphics::Bounds;
use std::hash::Hash;

#[derive(Debug)]
pub(crate) struct Window {
    cg: core_graphics::Window,
    ax: ax_ui::Window,
    bounds: Bounds,
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

    pub(crate) fn init(&mut self) -> Result<()> {
        println!("moving window {} to {:?}", self.cg.number(), self.bounds);
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
