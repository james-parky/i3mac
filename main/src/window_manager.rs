use crate::{
    container,
    display::Display,
    error::{Error, Result},
    event_loop::Event,
};
use core_graphics::{Direction, DisplayId, KeyCommand, WindowId};
use foundation::StatusBar;
use std::collections::HashMap;

pub(super) struct WindowManager {
    displays: HashMap<DisplayId, Display>,
    status_bars: HashMap<DisplayId, StatusBar>,
}

impl WindowManager {
    pub(super) fn new() -> Self {
        Self {
            displays: HashMap::new(),
            status_bars: HashMap::new(),
        }
    }

    pub(super) fn handle_event(&mut self, event: Event) {
        match event {
            Event::WindowAdded { display_id, window } => {
                self.handle_window_added(display_id, window);
            }
            Event::WindowRemoved {
                display_id,
                window_id,
            } => {
                self.handle_window_removed(display_id, window_id);
            }
            Event::WindowFocused { window_id } => {
                self.handle_window_focused(window_id);
            }
            Event::DisplayAdded {
                display_id,
                display,
            } => {
                self.handle_display_added(display_id, display);
            }
            Event::KeyCommand { command } => {
                self.handle_key_command(command);
            } // Event::Tick => {}
        }
    }

    fn handle_window_added(&mut self, display_id: DisplayId, window: core_graphics::Window) {
        if let Some(display) = self.displays.get_mut(&display_id) {
            match display.add_window(window.clone()) {
                Err(e) => println!("error adding: {:?}", e),
                Ok(_) => {
                    display.set_focused_window(window.number());
                }
            }
        }
    }

    fn handle_window_removed(&mut self, display_id: DisplayId, window_id: WindowId) {
        if let Some(display) = self.displays.get_mut(&display_id)
            && let Err(e) = display.remove_window(window_id)
        {
            println!("error removing window: {:?}", e);
        }
    }

    fn handle_window_focused(&mut self, window_id: WindowId) {
        for display in self.displays.values_mut() {
            if display.window_ids().contains(&window_id) {
                display.set_focused_window(window_id);
                return;
            }
        }
    }

    fn handle_display_added(&mut self, display_id: DisplayId, cg_display: core_graphics::Display) {
        let mut display = Display::new(cg_display.bounds);

        for window in cg_display.windows {
            if let Err(e) = display.add_window(window) {
                println!("error adding: {:?}", e);
            }
        }

        let bar = StatusBar::new(display_id, display.bounds());
        bar.display();

        self.status_bars.insert(display_id, bar);
        self.displays.insert(display_id, display);
    }

    fn handle_key_command(&mut self, command: KeyCommand) {
        match command {
            KeyCommand::NewTerminal => {
                open_terminal();
            }
            KeyCommand::CloseWindow => {
                // if let Err(e) = close_focused_window(&mut ctx.displays) {
                //     println!("Failed to close window: {:?}", e);
                // }
            }
            KeyCommand::Focus(direction) => {
                if let Err(e) = self.handle_focus_shift(direction) {
                    println!("Failed to shift focus: {:?}", e);
                }
            }
            KeyCommand::FocusDisplay(display_id) => {
                if let Some(display) = self.displays.get(&(display_id as usize).into()) {
                    if let Err(ref err) = display.focus() {
                        eprintln!("failed to focus display {display_id}: {err:?}")
                    }
                } else {
                    eprintln!("display {display_id} does not exist");
                }
            }
            KeyCommand::MoveWindowToDisplay(n) => {
                if let Err(e) = self.move_window_to_display(n) {
                    println!("Failed to move window: {:?}", e);
                }
            }
            KeyCommand::MoveWindow(_) => {
                // if let Ok(window_id) = ax_ui::Window::get_focused() {
                //     for display in ctx.displays.values_mut() {
                //         if display.move_window(window_id, direction) {
                //             break;
                //         }
                //     }
                // }
            }
            KeyCommand::ToggleVerticalSplit => {
                self.handle_split(container::Direction::Vertical);
            }
            KeyCommand::ToggleHorizontalSplit => {
                self.handle_split(container::Direction::Horizontal);
            }
            KeyCommand::ResizeWindow(direction) => {
                if let Ok(focused_window) = ax_ui::Window::try_get_focused() {
                    for display in self.displays.values_mut() {
                        if display.window_ids().contains(&focused_window)
                            && let Err(e) =
                                display.resize_window_in_direction(focused_window, &direction)
                        {
                            println!("error resizing window: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    fn handle_split(&mut self, direction: container::Direction) {
        if let Ok(window_id) = ax_ui::Window::try_get_focused() {
            for display in self.displays.values_mut() {
                if let Some(parent) = display.get_leaf_of_window_mut(window_id) {
                    match parent.split(direction) {
                        Ok(_) => {
                            display.set_focused_window(window_id);
                        }
                        Err(e) => println!("Error splitting: {:?}", e),
                    }
                    break;
                }
            }
        }
    }

    fn handle_focus_shift(&self, direction: Direction) -> Result<()> {
        let current_window_id = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

        for display in self.displays.values() {
            let unordered_windows = display.window_ids();

            let windows: Vec<WindowId> = unordered_windows.into_iter().collect();
            println!("Window {:?}", windows);

            if let Some(current_idx) = windows.iter().position(|&id| id == current_window_id) {
                // Found the display with this window
                let next_idx = match direction {
                    Direction::Left | Direction::Up => {
                        if current_idx == 0 {
                            windows.len() - 1
                        } else {
                            current_idx - 1
                        }
                    }
                    Direction::Right | Direction::Down => (current_idx + 1) % windows.len(),
                };

                let next_window_id = windows[next_idx];
                return display.focus_window(next_window_id);
            }
        }

        Err(Error::WindowNotFound)
    }

    fn move_window_to_display(&mut self, display_id: u64) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
        // Find the CG window info before removing
        let mut cg_window: Option<core_graphics::Window> = None;

        for (id, display) in self.displays.iter() {
            if let Some(window) = display.find_window(focused_window) {
                // TODO: lol
                if *id == (display_id as usize).into() {
                    return Err(Error::CannotMoveWindowToSameDisplay);
                }
                cg_window = Some(window.cg().clone());
                break;
            }
        }

        let cg_window = cg_window.ok_or(Error::WindowNotFound)?;

        // Remove from current display
        for display in self.displays.values_mut() {
            if display.remove_window(focused_window)? {
                break;
            }
        }

        let target_display = self
            .displays
            .get_mut(&(display_id as usize).into())
            .ok_or(Error::DisplayNotFound)?;

        target_display.add_window(cg_window)
    }
}

fn open_terminal() {
    use std::process::Command;
    let _ = Command::new("open")
        .arg("-n")
        .arg("-a")
        .arg("Terminal")
        .spawn();
}
