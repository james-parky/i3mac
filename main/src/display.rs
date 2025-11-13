use crate::{Error, Result, container, container::Container, window::Window};
use core_graphics::{Bounds, Direction, WindowId};
use std::collections::HashSet;

#[derive(Debug)]
pub(crate) struct Display {
    bounds: Bounds,
    root: Container,
    focused_window: Option<WindowId>,
}

impl Display {
    pub fn focus(&self) -> Result<()> {
        let to_focus = self.focused_window.unwrap_or(
            *self
                .window_ids()
                .iter()
                .nth(0)
                .ok_or(Error::CannotFocusEmptyDisplay)?,
        );

        self.focus_window(to_focus)
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub(crate) fn new(cg_bounds: Bounds) -> Self {
        // Core Graphics bounds do not include the Apple status bar, we need to
        // subtract the height of said bar to stop vertically split windows from
        // overlapping each other.
        const MENU_BAR_HEIGHT: f64 = 37.0;
        const STATUS_BAR_HEIGHT: f64 = 25.0;

        let bounds = Bounds {
            height: cg_bounds.height - MENU_BAR_HEIGHT - STATUS_BAR_HEIGHT,
            y: cg_bounds.y + MENU_BAR_HEIGHT,
            ..cg_bounds
        };

        Display {
            bounds,
            root: Container::Empty { bounds },
            focused_window: None,
        }
    }

    pub(crate) fn window_ids(&self) -> HashSet<WindowId> {
        self.root.window_ids()
    }

    pub(crate) fn cg_windows(&self) -> HashSet<&core_graphics::Window> {
        self.root.cg_windows()
    }
    pub fn find_window(&self, window_id: WindowId) -> Option<&Window> {
        self.root.find_window(window_id)
    }
    pub fn find_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        self.root.find_window_mut(window_id)
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Result<bool> {
        // let rem = self.root.remove_window(window_id)?;
        //
        // match self.focused_window {
        //     Some(window) if window == window_id => {
        //         self.focused_window = None;
        //     }
        //     _ => {}
        // }

        println!(
            "removing window {window_id:?} current focus: {:?}",
            self.focused_window
        );
        if self.focused_window == Some(window_id) {
            let new_focus = self.find_next_focus(window_id);
            println!("new focus afte rremoval : {new_focus:?}");
            self.focused_window = new_focus;
        }

        self.root.remove_window(window_id)
    }

    fn find_next_focus(&self, window_id: WindowId) -> Option<WindowId> {
        if let Some(sibling) = self.find_sibling(window_id) {
            return Some(sibling);
        }

        self.root
            .window_ids()
            .into_iter()
            .find(|&id| id != window_id)
    }

    fn find_sibling(&self, window_id: WindowId) -> Option<WindowId> {
        self.root.sibling_of(window_id)
    }

    pub fn add_window(&mut self, window: core_graphics::Window) -> Result<()> {
        // if let Some(focused_id) = self.focused_window {
        //     if let Some(container) = self.root.get_parent_of_window_mut(focused_id) {
        //         return container.add_window(window);
        //     }
        // }
        //
        // self.root.add_window(window)

        let window_id = window.number();

        if let Some(focused_id) = self.focused_window {
            println!("focused window: {:?}", focused_id);
            if let Some(container) = self.root.get_parent_of_window_mut(focused_id) {
                println!("container: {:?}", container);
                container.add_window(window)?;
                self.focused_window = Some(window_id);
                return Ok(());
            }
        }

        println!("adding to root");
        self.root.add_window(window)?;
        self.focused_window = Some(window_id);
        Ok(())
    }

    pub fn focus_window(&self, window_id: WindowId) -> Result<()> {
        if let Some(window) = self.root.find_window(window_id) {
            window.ax().try_focus().map_err(Error::AxUi)
        } else {
            Err(Error::WindowNotFound)
        }
    }

    pub fn get_parent_of_window(&mut self, window_id: WindowId) -> Option<&mut Container> {
        self.root.get_parent_of_window_mut(window_id)
    }

    pub fn get_leaf_of_window_mut(&mut self, window_id: WindowId) -> Option<&mut Container> {
        self.root.get_leaf_of_window_mut(window_id)
    }

    pub fn set_focused_window(&mut self, window_id: WindowId) {
        self.focused_window = Some(window_id);
    }

    pub fn resize_focused(&mut self, window_id: WindowId, direction: &Direction) -> Result<()> {
        const RESIZE_AMOUNT: f64 = 50.0;
        self.root
            .resize_window(window_id, direction, RESIZE_AMOUNT, self.bounds)
    }
}
