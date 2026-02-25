use crate::display::{Displays, LogicalDisplayId};
use crate::status_bar::StatusBar;
use crate::{
    container, display,
    error::{Error, Result},
    event_loop::Event,
    event_loop::EventLoop,
    log,
    log::Message::{
        FocusLogicalDisplayKeyCommand, MoveFocusedWindowToLogicalDisplayKeyCommand,
        OpenTerminalKeyCommand, ResizeWindowInDirectionKeyCommand, ShiftFocusInDirectionKeyCommand,
        ToggleHorizontalSplitKeyCommand, ToggleVerticalSplitKeyCommand,
        WindowMovedToLogicalDisplay, WindowResized, WindowSplitAlongAxis,
    },
    log::Message::{
        ReceivedKeyCommand, ReceivedWindowAddedEvent, ReceivedWindowFocusedEvent,
        ReceivedWindowRemovedEvent, WindowFocused,
    },
    log::{Level, Log, Logger},
    window::Window,
};
use core_foundation::{CFRunLoopGetCurrent, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::{Bounds, Direction, DisplayId, KeyCommand, KeyboardHandler, WindowId};
use foundation::{WorkspaceEvent, WorkspaceObserver};
use log::Message::{WindowAdded, WindowRemoved};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::channel,
};

#[derive(Default, Copy, Clone, Debug)]
pub struct Config {
    pub window_padding: Option<f64>,
    log_level: Level,
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
                "--log-level" => {
                    let level: Level = args
                        .next()
                        .expect("expected one of {info, warn, error, trace}  after --log-level")
                        .as_str()
                        .try_into()
                        .expect("expected one of {info, warn, error, trace}  after --log-level");
                    ret.log_level = level;
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
    windows: HashMap<WindowId, Window>,
    displays: Displays,
    floating_windows: HashSet<core_graphics::Window>,
    logger: Logger,
    config: Config,
    status_bars: HashMap<DisplayId, StatusBar>,
}

impl WindowManager {
    // To create a new WindowManager:
    //  - Find all active CoreGraphics displays
    //  - For each display:
    //     - Create a physical display and give them a logical display from the pool
    //     - For each window: give the associate window ID to the current logical display. If the
    //       logical display cannot fit the window (due to some minimum size constraint etc.) then
    //       create a new logical display on the physical display, then add the window the
    //       WindowManagers set of managed windows
    //  - Move all managed windows to where they should be
    pub(super) fn new(config: Config) -> Result<Self> {
        let logger =
            Logger::try_new("/dev/stdout", config.log_level).expect("failed to create logger");

        let mut wm = Self {
            windows: Default::default(),
            displays: Default::default(),
            floating_windows: Default::default(),
            logger,
            config,
            status_bars: Default::default(),
        };

        for (id, cg_display) in core_graphics::Display::all().map_err(Error::CoreGraphics)? {
            // First CoreGraphics display detected is chosen to be the active physical display
            wm.displays
                .add_physical(id.into(), cg_display.bounds, config.into());

            // For each window in the CoreGraphics display:
            //  - Try to insert it into the current logical display. If it can't fit it, create a
            //    new logical display, on the same physical display as previous.
            for window in cg_display.windows {
                let window_id = window.number();
                let w = Window::try_from(window)?;
                let min_size = w.ax().min_size().unwrap_or_default();
                let cw = container::Window {
                    id: window_id,
                    min_width: min_size.width,
                    min_height: min_size.height,
                };

                wm.displays.add_window(cw)?;
                wm.windows.insert(window_id, w);
            }

            let lids = wm.displays.logical_ids(id.into());
            let status_bar = StatusBar::new(lids.into_iter().collect(), cg_display.bounds);
            wm.status_bars.insert(id, status_bar);
        }

        wm.update_status_bars();
        wm.apply_layout()?;
        Ok(wm)
    }

    fn update_status_bars(&mut self) {
        let active_id = self.displays.active_logical_display_id();
        for sb in self.status_bars.values_mut() {
            sb.draw(active_id);
        }
    }

    fn apply_layout(&mut self) -> Result<()> {
        for pd in self.displays.physical_displays().values() {
            for (id, bounds) in pd.active_window_bounds() {
                // ← only active LD
                if let Some(window) = self.windows.get_mut(&id) {
                    window.update_bounds(bounds)?;
                    window.init()?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn run(&mut self) -> Result<()> {
        let (key_tx, key_rx) = channel::<KeyCommand>();
        let keyboard = KeyboardHandler::new(key_tx).expect("failed to create keyboard handler");

        let mut event_loop = EventLoop::new(key_rx);

        // Safety:
        //  - The `run_loop` supplied to `KeyboardHandler::add_run_loop()` is valid
        //    as it was returned by the library function `CFRunLoopGetCurrent()`.
        unsafe { keyboard.add_to_run_loop(CFRunLoopGetCurrent(), kCFRunLoopDefaultMode) }
            .expect("failed to add keyboard to run loop");

        let (workspace_tx, workspace_rx) = channel::<WorkspaceEvent>();
        let _workspace_observer = WorkspaceObserver::new(workspace_tx);
        loop {
            println!("windows: {:?}", self.windows.keys());
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) }

            let mut should_poll_windows = false;
            if let Ok(_event) = workspace_rx.try_recv() {
                should_poll_windows = true;
            }

            let keyboard_events = event_loop.poll_keyboard();
            if !keyboard_events.is_empty() {
                should_poll_windows = true;
                for event in keyboard_events {
                    self.handle_event(event);
                }
            }

            if should_poll_windows {
                for event in event_loop.poll_windows() {
                    self.handle_event(event);
                }
            }

            // self.reset_windows();
        }
    }

    // pub(super) fn reset_windows(&mut self) {
    //     for display in self.physical_displays.values_mut() {
    //         for window in display.windows_mut() {
    //             if let Err(e) = window.update_bounds(*window.bounds()) {
    //                 eprintln!(
    //                     "could not set window {:?} back to designated size and location: {e:?}",
    //                     window.cg().number()
    //                 );
    //             }
    //         }
    //     }
    // }

    pub(super) fn handle_event(&mut self, event: Event) {
        match event {
            Event::WindowAdded { display_id, window } => {
                ReceivedWindowAddedEvent(display_id, window.number()).log(&mut self.logger);

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
                ReceivedWindowRemovedEvent(display_id, window_id).log(&mut self.logger);

                if let Err(e) = self.handle_window_removed(display_id, window_id) {
                    eprintln!("failed to remove window: {e:?}");
                }
            }

            // Event::DisplayAdded {
            //     display_id,
            //     display,
            // } => {
            //     self.handle_display_added(display_id, display);
            // }
            Event::KeyCommand { command } => {
                ReceivedKeyCommand(command).log(&mut self.logger);

                self.handle_key_command(command);
            }
            Event::WindowFocused { window_id } => {
                ReceivedWindowFocusedEvent(window_id).log(&mut self.logger);
                if let Err(e) = self.handle_window_focus(window_id) {
                    eprintln!("failed to focus window: {e:?}");
                }
            }
            _ => {}
        }
    }

    fn handle_window_focus(&mut self, window_id: WindowId) -> Result<()> {
        if let Some(pid) = self.displays.display_of_window(window_id) {
            self.displays.set_active_physical_display(pid);
        }

        Ok(())
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
        let window_id = cg_window.number();
        let window = Window::try_from(cg_window)?;
        let min_size = window.ax().min_size().unwrap_or_default();
        let cw = container::Window {
            id: window_id,
            min_width: min_size.width,
            min_height: min_size.height,
        };

        let res = self.displays.add_window(cw)?;

        let lid = match res {
            display::AddWindowResult::Active(lid) => lid,
            display::AddWindowResult::Overflow(lid) => {
                self.windows.insert(window_id, window);
                self.windows
                    .get_mut(&window_id)
                    .unwrap()
                    .ax()
                    .minimise()
                    .map_err(Error::AxUi)?;

                let status_bar = self.status_bars.get_mut(&display_id).unwrap();
                status_bar.add_logical_id(lid);
                self.update_status_bars();
                return Ok(());
            }
        };

        let status_bar = self.status_bars.get_mut(&display_id).unwrap();
        status_bar.add_logical_id(lid);
        self.windows.insert(window_id, window);
        self.update_status_bars();
        self.apply_layout()?;
        Ok(())
    }

    fn handle_window_removed(&mut self, display_id: DisplayId, window_id: WindowId) -> Result<()> {
        if self
            .floating_windows
            .iter()
            .any(|w| w.number() == window_id)
        {
            return Ok(());
        }

        self.displays.remove_window(display_id.into(), window_id)?;

        WindowRemoved(
            display_id.into(),
            self.displays.active_logical_display_id(),
            window_id,
        )
        .log(&mut self.logger);

        self.windows.remove(&window_id);
        self.apply_layout()?;

        Ok(())
    }

    fn handle_key_command(&mut self, command: KeyCommand) {
        match command {
            KeyCommand::NewTerminal => {
                OpenTerminalKeyCommand.log(&mut self.logger);
                open_terminal();
            }
            KeyCommand::CloseWindow => {
                // if let Err(e) = close_focused_window(&mut ctx.displays) {
                //     println!("Failed to close window: {:?}", e);
                // }
            }
            KeyCommand::Focus(direction) => {
                ShiftFocusInDirectionKeyCommand(direction).log(&mut self.logger);
                if let Err(e) = self.handle_focus_shift(direction) {
                    println!("Failed to shift focus: {:?}", e);
                }
            }
            KeyCommand::FocusDisplay(display_id) => {
                FocusLogicalDisplayKeyCommand(display_id.into()).log(&mut self.logger);
                if let Err(e) = self.handle_focus_logical_display(display_id.into()) {
                    eprintln!("failed to focus display: {e:?}");
                }
            }
            KeyCommand::MoveWindowToDisplay(n) => {
                MoveFocusedWindowToLogicalDisplayKeyCommand((n as usize).into())
                    .log(&mut self.logger);
                if let Err(e) = self.handle_move_focused_window_to_display((n as usize).into()) {
                    println!("Failed to move window: {:?}", e);
                }
            }
            // KeyCommand::MoveWindow(_) => {
            //     if let Ok(window_id) = ax_ui::Window::get_focused() {
            //         for display in ctx.displays.values_mut() {
            //             if display.move_window(window_id, direction) {
            //                 break;
            //             }
            //         }
            //     }
            // }
            KeyCommand::ToggleVerticalSplit => {
                ToggleVerticalSplitKeyCommand.log(&mut self.logger);
                if let Err(e) = self.handle_split(container::Axis::Vertical) {
                    eprintln!("failed to split container vertically: {e:?}");
                }
            }
            KeyCommand::ToggleHorizontalSplit => {
                ToggleHorizontalSplitKeyCommand.log(&mut self.logger);
                if let Err(e) = self.handle_split(container::Axis::Horizontal) {
                    eprintln!("failed to split container horizontally: {e:?}");
                }
            }
            KeyCommand::ResizeWindow(direction) => {
                ResizeWindowInDirectionKeyCommand(direction).log(&mut self.logger);
                if let Err(e) = self.handle_resize(direction) {
                    eprintln!("failed to resize window: {e:?}");
                }
            }

            // KeyCommand::ToggleFloating => {
            //     ToggleWindowFloatingKeyCommand.log(&mut self.logger);
            //     if let Err(e) = self.handle_toggle_floating() {
            //         eprintln!("failed to toggle floating: {e:?}");
            //     }
            // }
            _ => {}
        }
    }

    // To handle toggling a window to be floating:
    //  1. Get the currently focused window.
    //  2. If the currently focused window is already floating, add it to the
    //     active physical display, and mark it as not floating, else remove it
    //     from the active physical display, and mark it as floating.
    // fn handle_toggle_floating(&mut self) -> Result<()> {
    //     let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
    //
    //     let cg_window = self
    //         .floating_windows
    //         .iter()
    //         .find(|w| w.number() == focused_window)
    //         .cloned();
    //
    //     if let Some(cg_window) = cg_window {
    //         self.floating_windows.remove(&cg_window);
    //         self.active_physical_display_mut().add_window(cg_window)?;
    //         WindowMadeManaged(focused_window).log(&mut self.logger);
    //     } else {
    //         let removed = self
    //             .active_physical_display_mut()
    //             .remove_window(focused_window)?
    //             .ok_or(Error::CouldNotRemoveWindow)?;
    //
    //         // Sanity check
    //         if removed.cg().number() != focused_window {
    //             panic!("just removed a window that we shouldn't have");
    //         }
    //
    //         self.floating_windows.insert(removed.cg().clone());
    //         WindowMadeFloating(focused_window).log(&mut self.logger);
    //     }
    //
    //     Ok(())
    // }

    fn handle_resize(&mut self, direction: Direction) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
        WindowResized(focused_window, direction).log(&mut self.logger);
        self.displays
            .active_physical_display_mut()
            .set_focused_window(focused_window);
        self.displays
            .active_physical_display_mut()
            .resize_focused_window(direction)?;
        self.apply_layout()
    }

    fn handle_split(&mut self, axis: container::Axis) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
        self.displays.split(axis)?;
        WindowSplitAlongAxis(focused_window, axis).log(&mut self.logger);
        self.apply_layout()
    }

    fn handle_focus_shift(&mut self, direction: Direction) -> Result<()> {
        ShiftFocusInDirectionKeyCommand(direction).log(&mut self.logger);
        self.displays
            .active_physical_display_mut()
            .shift_focus(direction)
            .map(|_| ())
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
    fn handle_move_focused_window_to_display(&mut self, target: LogicalDisplayId) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

        // Find the physical display that owns the currently focused window.
        let owner = self.displays.display_of_window(focused_window).unwrap();

        let min_size = self
            .windows
            .get(&focused_window)
            .unwrap()
            .ax()
            .min_size()
            .unwrap_or_default();

        let window = container::Window {
            id: focused_window,
            min_width: min_size.width,
            min_height: min_size.height,
        };

        self.displays.remove_window(owner, focused_window)?;

        self.displays.add_window_to_logical(window, target)?;

        self.displays.focus_display(target);

        self.apply_layout()?;

        WindowMovedToLogicalDisplay(focused_window, target).log(&mut self.logger);
        Ok(())
    }

    /// Focus a logical display by ID.
    ///
    /// - If the logical display does not already exist, create it on the
    ///   currently active physical display, and add it to the window manager's
    ///   active logical display set.
    ///
    /// - If the previously active logical display on the concerned physical
    ///   display has no window, remove it from the physical display's logical
    ///   display set, and from the window manager's active logical display set.
    ///   Otherwise, minimise all windows on the previous logical display.
    ///
    /// - If the logical display did previously exist, maximise all windows on
    ///   the new active logical display.
    ///
    /// We only need to focus the active display if the logical display already
    /// exists. Ones that have no windows will have been deleted when they were
    /// last focused off of.
    fn handle_focus_logical_display(&mut self, new_lid: LogicalDisplayId) -> Result<()> {
        let current_lid = self.displays.active_logical_display_id();
        if current_lid == new_lid {
            return Ok(());
        }

        // Find which PD owns the current active LD (always exists).
        let current_pid = *self
            .displays
            .physical_displays()
            .iter()
            .find(|(_, pd)| pd.has_logical_display(current_lid))
            .map(|(pid, _)| pid)
            .unwrap();

        // Find which PD owns the target LD. If none, create it on the current PD.
        let target_pid = self
            .displays
            .physical_displays()
            .iter()
            .find(|(_, pd)| pd.has_logical_display(new_lid))
            .map(|(pid, _)| *pid)
            .unwrap_or_else(|| {
                // create_logical_display now takes the specific lid we want.
                self.displays.create_logical_display(current_pid, new_lid);
                current_pid
            });

        let same_pd = current_pid == target_pid;

        // Collect the current LD's windows before the switch.
        let current_window_ids: Vec<_> = self
            .displays
            .physical_displays()
            .get(&current_pid)
            .unwrap()
            .active_logical_display()
            .unwrap()
            .window_ids()
            .into_iter()
            .collect();

        if same_pd {
            for wid in &current_window_ids {
                if let Some(w) = self.windows.get_mut(wid) {
                    w.ax().minimise().map_err(Error::AxUi)?;
                }
            }
        }

        let removed_lid = self.displays.switch_logical_display(target_pid, new_lid)?;
        if let Some(dead_lid) = removed_lid {
            if let Some(sb) = self.status_bars.get_mut(&DisplayId::from(target_pid)) {
                sb.remove_logical_id(dead_lid);
            }
        }

        self.displays.focus_display(new_lid);

        let new_window_ids: Vec<_> = self
            .displays
            .physical_displays()
            .get(&target_pid)
            .unwrap()
            .active_logical_display()
            .unwrap()
            .window_ids()
            .into_iter()
            .collect();

        if same_pd {
            for wid in &new_window_ids {
                if let Some(w) = self.windows.get_mut(wid) {
                    w.ax().unminimise().map_err(Error::AxUi)?;
                }
            }
        }

        if let Some(sb) = self.status_bars.get_mut(&DisplayId::from(target_pid)) {
            sb.add_logical_id(new_lid);
        }

        self.apply_layout()?;
        self.update_status_bars();
        Ok(())
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
