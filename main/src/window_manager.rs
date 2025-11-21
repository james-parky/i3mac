use crate::{
    container,
    display::{PhysicalDisplay, VirtualDisplayId},
    error::{Error, Result},
    event_loop::Event,
};
use core_graphics::{Direction, DisplayId, KeyCommand, Window, WindowId};
use std::collections::HashMap;

pub(super) struct WindowManager {
    physical_displays: HashMap<DisplayId, PhysicalDisplay>,
    active_physical_display: DisplayId,
}

impl WindowManager {
    pub(super) fn new() -> Self {
        // We want a physical display for each display reported by Core Graphics
        let mut physical_displays = HashMap::new();
        // TODO: unwrap
        let detected_physical_displays = core_graphics::Display::all().unwrap();

        for (id, display) in detected_physical_displays {
            let physical = PhysicalDisplay::new(id, display);
            physical_displays.insert(id, physical);
        }

        // Display with the lowest Core Graphics ID is chosen to be initially
        // focused
        // TODO: horrible error on not detecting any displays
        let active_physical_display = core_graphics::Display::main_display();

        // TODO: error
        let _ = physical_displays
            .get(&active_physical_display)
            .unwrap()
            .focus();

        Self {
            physical_displays,
            active_physical_display,
        }
    }

    pub(super) fn handle_event(&mut self, event: Event) {
        match event {
            Event::WindowAdded { display_id, window } => {
                println!("Window added: {:?}", display_id);
                if let Err(e) = self.handle_window_added(display_id, window) {
                    eprintln!("failed to add window: {e:?}");
                }
            }
            // TODO: Need to not trigger this from windows that disappear due to
            //       switching virtual display
            Event::WindowRemoved {
                display_id,
                window_id,
            } => {
                if let Err(e) = self.handle_window_removed(display_id, window_id) {
                    eprintln!("failed to remove window: {e:?}");
                }
            }
            Event::WindowFocused { window_id } => {
                self.handle_window_focused(window_id);
            }
            // Event::DisplayAdded {
            //     display_id,
            //     display,
            // } => {
            //     self.handle_display_added(display_id, display);
            // }
            Event::KeyCommand { command } => {
                self.handle_key_command(command);
            }
            _ => {}
        }
    }

    // When a new window has been detected, add it to the previously focused
    // window's parent split. Therefore:
    //  1. Get the physical display that said window is on.
    //  2. Add window to it.
    fn handle_window_added(&mut self, display_id: DisplayId, window: Window) -> Result<()> {
        self.physical_displays
            .get_mut(&display_id)
            .ok_or(Error::DisplayNotFound)?
            .add_window(window)
    }

    fn handle_window_removed(&mut self, display_id: DisplayId, window_id: WindowId) -> Result<()> {
        match self
            .physical_displays
            .get_mut(&display_id)
            .ok_or(Error::DisplayNotFound)?
            .remove_window(window_id)
        {
            Err(e) => Err(e),
            Ok(false) => Err(Error::CouldNotRemoveWindow),
            Ok(true) => Ok(()),
        }
    }

    fn handle_window_focused(&mut self, window_id: WindowId) {
        for (id, display) in self.physical_displays.iter_mut() {
            if display.window_ids().contains(&window_id) {
                self.active_physical_display = *id;
                return;
            }
        }
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
                if let Err(e) = self.handle_focus_virtual_display(display_id.into()) {
                    eprintln!("failed to focus display: {e:?}");
                }
            }
            KeyCommand::MoveWindowToDisplay(n) => {
                if let Err(e) = self.handle_move_focused_window_to_display((n as usize).into()) {
                    println!("Failed to move window: {:?}", e);
                }
            }
            // KeyCommand::MoveWindow(_) => {
            //     // if let Ok(window_id) = ax_ui::Window::get_focused() {
            //     //     for display in ctx.displays.values_mut() {
            //     //         if display.move_window(window_id, direction) {
            //     //             break;
            //     //         }
            //     //     }
            //     // }
            // }
            KeyCommand::ToggleVerticalSplit => {
                if let Err(e) = self.handle_split(container::Direction::Vertical) {
                    eprintln!("failed to split container vertically: {e:?}");
                }
            }
            KeyCommand::ToggleHorizontalSplit => {
                if let Err(e) = self.handle_split(container::Direction::Horizontal) {
                    eprintln!("failed to split container horizontally: {e:?}");
                }
            }
            KeyCommand::ResizeWindow(direction) => {
                if let Err(e) = self.handle_resize(direction) {
                    eprintln!("failed to resize window: {e:?}");
                }
            }

            _ => {}
        }
    }

    fn handle_resize(&mut self, direction: Direction) -> Result<()> {
        self.physical_displays
            .get_mut(&self.active_physical_display)
            .unwrap()
            .resize_focused_window(direction)
    }

    fn handle_split(&mut self, direction: container::Direction) -> Result<()> {
        self.physical_displays
            .get_mut(&self.active_physical_display)
            .unwrap()
            .split(direction)
    }

    // When handling a focus shift, only allow movement within the currently
    // active physical display, so delegate to that.
    fn handle_focus_shift(&mut self, direction: Direction) -> Result<()> {
        println!("focusing shift towards {direction:?}");
        self.physical_displays
            .get_mut(&self.active_physical_display)
            .ok_or(Error::DisplayNotFound)?
            .handle_focus_shift(direction)
    }

    fn handle_move_focused_window_to_display(
        &mut self,
        target_virtual_display_id: VirtualDisplayId,
    ) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
        // Find the CG window info before removing
        let mut cg_window: Option<core_graphics::Window> = None;

        for display in self.physical_displays.values() {
            if let Some(window) = display.active_display().find_window(focused_window) {
                cg_window = Some(window.cg().clone());
                break;
            }
        }

        let cg_window = cg_window.ok_or(Error::WindowNotFound)?;

        if !self
            .physical_displays
            .get_mut(&self.active_physical_display)
            .unwrap()
            .remove_window(focused_window)?
        {
            return Err(Error::CouldNotRemoveWindow);
        }

        // find physical display that owns virtual_display_id
        let mut target_physical_display = None;
        for display in self.physical_displays.values_mut() {
            if display.has_virtual_display(target_virtual_display_id) {
                target_physical_display = Some(display);
                break;
            }
        }

        // if we didnt find it, create new virtual display on this phyiscal display before adding
        if target_physical_display.is_none() {
            self.physical_displays
                .get_mut(&self.active_physical_display)
                .unwrap()
                .create_virtual_display(target_virtual_display_id);
            target_physical_display = Some(
                self.physical_displays
                    .get_mut(&self.active_physical_display)
                    .unwrap(),
            );
        }

        target_physical_display
            .unwrap()
            .add_window_to_virtual(cg_window, target_virtual_display_id)?;

        Ok(())
    }

    fn handle_focus_virtual_display(&mut self, virtual_id: VirtualDisplayId) -> Result<()> {
        let mut target_physical_id: Option<DisplayId> = None;

        // find the physical display that contains the target virtual id
        for (physical_id, physical) in &self.physical_displays {
            if physical.has_virtual_display(virtual_id) {
                target_physical_id = Some(*physical_id);
                break;
            }
        }

        match target_physical_id {
            None => {
                // virtual id does not exist; create it on this physical display
                let focused_physical_id = self.get_focused_physical_display()?;

                let physical = self
                    .physical_displays
                    .get_mut(&focused_physical_id)
                    .unwrap();

                physical.create_virtual_display(virtual_id);
                physical.switch_to(virtual_id)?;
                Ok(())
            }
            Some(physical_id) => {
                let physical = self.physical_displays.get_mut(&physical_id).unwrap();
                physical.switch_to(virtual_id)?;
                // Only need to focus the active display if the virtual display
                // already exists. Ones that have no windows will have been
                // deleted when they were last focused off of.
                physical.active_display().refocus()?;
                Ok(())
            }
        }
    }

    fn get_focused_physical_display(&self) -> Result<DisplayId> {
        if let Ok(focused_window) = ax_ui::Window::try_get_focused() {
            for (physical_id, physical) in &self.physical_displays {
                if physical
                    .active_virtual_display()
                    .ok_or(Error::DisplayNotFound)?
                    .window_ids()
                    .contains(&focused_window)
                {
                    return Ok(*physical_id);
                }
            }
        }

        self.physical_displays
            .keys()
            .next()
            .copied()
            .ok_or(Error::DisplayNotFound)
    }
}

// When opening a new Terminal via `open`, the OS will sometimes the window in
// the same "state" as the previously focused window of the same application (if
// one exists), i.e. opening a window already minimised. If this happens, Core
// Graphics won't detect it has been happened, and therefore no will the window
// manager. To prevent this, do some faffing via AppleScript.
fn open_terminal() {
    use std::process::Command;
    let _ = Command::new("open")
        .arg("-n")
        .arg("-a")
        .arg("Terminal")
        .status();

    let apple_script = r#"
        tell application "System Events"
            tell process "Terminal"
                set windowList to every window
                if (count of windowList) > 0 then
                    set frontWindow to item 1 of windowList
                    if value of attribute "AXMinimized" of frontWindow is true then
                        set value of attribute "AXMinimized" of frontWindow to false
                    end if
                end if
            end tell
        end tell
    "#;

    let _ = Command::new("osascript")
        .arg("-e")
        .arg(apple_script)
        .status();
}
