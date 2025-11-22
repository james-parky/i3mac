use crate::{
    container,
    display::{LogicalDisplayId, PhysicalDisplay},
    error::{Error, Result},
    event_loop::Event,
};
use core_graphics::{Direction, DisplayId, KeyCommand, WindowId};
use std::collections::HashMap;

#[derive(Default, Copy, Clone, Debug)]
pub struct Config {
    pub window_padding: Option<f64>,
}

impl Config {
    pub fn must_parse() -> Self {
        let mut ret = Self::default();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--padding" => {
                    let padding = args
                        .next()
                        .expect("expected a usize value after --padding")
                        .parse::<usize>()
                        .expect("expected a usize value after --padding");
                    ret.window_padding = Some(padding as f64);
                }
                unknown => {
                    panic!("{}", format!("unknown argument: {unknown}"));
                }
            }
        }

        ret
    }
}

pub(super) struct WindowManager {
    physical_displays: HashMap<DisplayId, PhysicalDisplay>,
    active_physical_display_id: DisplayId,
}

impl WindowManager {
    pub(super) fn new(config: Config) -> Self {
        // We want a physical display for each display reported by Core Graphics
        let mut physical_displays = HashMap::new();
        // TODO: unwrap
        let detected_physical_displays = core_graphics::Display::all().unwrap();

        for (id, display) in detected_physical_displays {
            let physical = PhysicalDisplay::new(id, display, config.into());
            physical_displays.insert(id, physical);
        }

        // Display with the lowest Core Graphics ID is chosen to be initially
        // focused
        // TODO: horrible error on not detecting any displays
        let active_physical_display_id = core_graphics::Display::main_display();

        // TODO: error
        let _ = physical_displays
            .get(&active_physical_display_id)
            .unwrap()
            .focus();

        Self {
            physical_displays,
            active_physical_display_id,
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
            //       switching logical display
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
    fn handle_window_added(
        &mut self,
        display_id: DisplayId,
        cg_window: core_graphics::Window,
    ) -> Result<()> {
        self.physical_displays
            .get_mut(&display_id)
            .ok_or(Error::DisplayNotFound)?
            .add_window(cg_window)
    }

    fn handle_window_removed(&mut self, display_id: DisplayId, window_id: WindowId) -> Result<()> {
        match self
            .physical_displays
            .get_mut(&display_id)
            .ok_or(Error::DisplayNotFound)?
            .remove_window(window_id)?
        {
            false => Err(Error::CouldNotRemoveWindow),
            true => Ok(()),
        }
    }

    fn handle_window_focused(&mut self, window_id: WindowId) {
        if let Some((id, _)) = self
            .physical_displays
            .iter_mut()
            .find(|(_, display)| display.window_ids().contains(&window_id))
        {
            self.active_physical_display_id = *id;
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
                if let Err(e) = self.handle_focus_logical_display(display_id.into()) {
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
                if let Err(e) = self.handle_split(container::Axis::Vertical) {
                    eprintln!("failed to split container vertically: {e:?}");
                }
            }
            KeyCommand::ToggleHorizontalSplit => {
                if let Err(e) = self.handle_split(container::Axis::Horizontal) {
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
        self.active_physical_display_mut()
            .resize_focused_window(direction)
    }

    fn handle_split(&mut self, direction: container::Axis) -> Result<()> {
        self.active_physical_display_mut().split(direction)
    }

    fn handle_focus_shift(&mut self, direction: Direction) -> Result<()> {
        self.active_physical_display_mut().shift_focus(direction)
    }

    /// Move the currently focused window from one logical display to another.
    ///
    /// If the target logical display ID does not exist, first create it on the
    /// currently active physical display, then move the window there.
    // In order to do this:
    //  1. Get the currently focused window.
    //  2. Find the physical display that currently owns the focused window, and
    //     the Core Graphics window that corresponds to it.
    //  3. Try to remove the window from the source physical display.
    //  4. Find the target physical display ID from the target logical display
    //     ID. If no physical display manages said logical display ID, create it
    //     on the currently active physical display.
    //  5. Try to add the removed Core Graphics window to the target logical
    //     display.
    fn handle_move_focused_window_to_display(
        &mut self,
        target_logical_display_id: LogicalDisplayId,
    ) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

        // Find the physical display that owns the currently focused window.
        let (source_physical_display_id, cg_window) = self
            .physical_displays
            .iter()
            .find_map(|(id, display)| {
                display
                    .active_display()
                    .find_window(focused_window)
                    .map(|w| (*id, w.cg().clone()))
            })
            .ok_or(Error::WindowNotFound)?;

        if !self
            .physical_displays
            .get_mut(&source_physical_display_id)
            .unwrap()
            .remove_window(focused_window)?
        {
            return Err(Error::CouldNotRemoveWindow);
        }

        let mut target_physical_display = None;
        for display in self.physical_displays.values_mut() {
            if display.has_logical_display(target_logical_display_id) {
                target_physical_display = Some(display);
                break;
            }
        }

        if target_physical_display.is_none() {
            self.active_physical_display_mut()
                .create_logical_display(target_logical_display_id);
            target_physical_display = Some(self.active_physical_display_mut());
        }

        target_physical_display
            .unwrap()
            .add_window_to_logical(cg_window, target_logical_display_id)?;

        Ok(())
    }

    /// Focus a logical display by ID.
    ///
    /// If the logical display does not already exist, create it on the
    /// currently active physical display.
    ///
    /// We only need to focus the active display if the logical display already
    /// exists. Ones that have no windows will have been deleted when they were
    /// last focused off of.
    fn handle_focus_logical_display(&mut self, logical_id: LogicalDisplayId) -> Result<()> {
        let target_physical_display_id = self.physical_displays.iter().find_map(|(id, display)| {
            if display.has_logical_display(logical_id) {
                Some(*id)
            } else {
                None
            }
        });

        match target_physical_display_id {
            None => {
                let focused_physical_id = self.try_get_focused_display_id()?;

                let physical = self
                    .physical_displays
                    .get_mut(&focused_physical_id)
                    .unwrap();

                physical.create_logical_display(logical_id);
                physical.switch_to(logical_id)?;
            }
            Some(physical_id) => {
                let physical = self.physical_displays.get_mut(&physical_id).unwrap();
                physical.switch_to(logical_id)?;
                physical.active_display().refocus()?;
            }
        }

        Ok(())
    }

    /// Try to get the ID of the physical display that owns the currently
    /// focused window.
    fn try_get_focused_display_id(&self) -> Result<DisplayId> {
        if let Ok(focused_window) = ax_ui::Window::try_get_focused() {
            for (physical_id, physical) in &self.physical_displays {
                if physical
                    .active_logical_display()
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

    fn active_physical_display_mut(&mut self) -> &mut PhysicalDisplay {
        self.physical_displays
            .get_mut(&self.active_physical_display_id)
            .unwrap()
    }
}

/// Open a new "Terminal" application window.
///
/// When opening a new Terminal via `open -n -a Terminal`, the OS will sometimes
/// open the window in the same "state" as the previously focused window of the
/// same application (if one exists), i.e. opening a window already minimised.
/// If this happens, Core Graphics won't detect it has opened, and therefore nor
/// will the window manager. To prevent this, do some faffing via AppleScript.
// TODO: Make the terminal application used configurable
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
