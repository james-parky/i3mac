use crate::{
    config::Config,
    container,
    ctl::{CTL_SOCK, WmToCtlMessage},
    display::{self, Displays, logical, physical},
    error::{Error, Result},
    event_loop::{self, EventLoop},
    log::{
        Log, Logger,
        Message::{
            FocusLogicalDisplayKeyCommand, MoveFocusedWindowToLogicalDisplayKeyCommand,
            OpenTerminalKeyCommand, ReceivedKeyCommand, ReceivedWindowAddedEvent,
            ReceivedWindowFocusedEvent, ReceivedWindowRemovedEvent,
            ResizeWindowInDirectionKeyCommand, ShiftFocusInDirectionKeyCommand,
            ToggleHorizontalSplitKeyCommand, ToggleVerticalSplitKeyCommand,
            ToggleWindowFloatingKeyCommand, WindowAdded, WindowMadeFloating, WindowMadeManaged,
            WindowMovedToLogicalDisplay, WindowRemoved, WindowResized, WindowSplitAlongAxis,
        },
    },
    poll::{
        AsKEvent, ChannelSource, Event, KeyboardHandler, Mux, Timer, WorkspaceEvent,
        WorkspaceObserver,
    },
    status_bar::StatusBar,
    window::Window,
};
use core_foundation::{CFRunLoopGetCurrent, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::{Direction, DisplayId, KeyCommand, WindowId};
use foundation::Colour;
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    os::unix::net::UnixListener,
    time::Duration,
};

// Arbitrary reasonable constant that stop windows getting too
// small. When this value is too small, the OS doesn't let the
// smaller window get smaller, but this code will make the larger
// window get larger and thus they overlap.
pub(crate) const MIN_WINDOW_SIZE: f64 = 200.0;
pub(crate) const RESIZE_AMOUNT: f64 = 50.0;

pub struct WindowManager {
    /// A map between window IDs reported by CoreGraphics, and our managed
    /// window objects.
    windows: HashMap<WindowId, Window>,
    /// The delegate display manager.
    displays: Displays,
    /// A set of window IDs that have been toggled floating by the user. These
    /// windows are kept track of, but not managed, and therefore not included
    /// in container bounds calculations.
    floating_windows: HashSet<WindowId>,
    /// A set of window IDs that have been minimised by the window manager but
    /// are still under management. When a window is minimised, either through
    /// user interaction, or the AXUI API, Core Graphics stops reporting its
    /// window ID. This causes issues with the minimisation/un-minimisation
    /// process performed during logical display focus shift; so keep track.
    minimised_windows: HashSet<WindowId>,
    /// A logger to that produces logs prefixed with "WM".
    logger: Logger,
    /// Config for the window manager.
    config: Config,
    /// A set of status bars, one per physical display reported by
    /// Core Graphics. These hold information about what logical displays exist
    /// on each physical display, and which one has global focus.
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
    pub fn new(config: Config) -> Result<Self> {
        let mut wm = Self {
            windows: Default::default(),
            displays: Displays::new(),
            floating_windows: Default::default(),
            minimised_windows: Default::default(),
            logger: Logger::try_new("/dev/stdout", config.log_level, "WM".into())
                .map_err(Error::CreateLogger)?,
            config,
            status_bars: Default::default(),
        };

        for (id, cg_display) in core_graphics::Display::all().map_err(Error::CoreGraphics)? {
            // First CoreGraphics display detected is chosen to be the active physical display
            wm.displays
                .add_physical(id.into(), cg_display.bounds, config.into());

            for window in cg_display.windows {
                wm.start_managing_window(window)?;
            }

            let lids: Vec<_> = wm.displays.logical_ids(id.into()).into_iter().collect();
            let status_bar = StatusBar::new(lids, cg_display.bounds, Colour::Clear);

            wm.status_bars.insert(id, status_bar);
        }

        wm.update_status_bars();
        wm.apply_layout()?;
        Ok(wm)
    }

    /// Start managing a window reported by Core Graphics.
    // To start managing a window:
    //  1. Create a `main::window::Window` from the `WindowId` provided by the
    //    supplied `core_graphics::Window`.
    //  2. Try to get minimum bounds for said window, defaulting on error.
    //  3. Create a `container::Window`: contains necessary information for
    //     placing a window in a container, i.e. `WindowId` and minimum bounds.
    //  4. Add it to some display, delegating to the `WindowManager`'s display
    //    manager, `Displays`.
    //  5. Add it to the `WindowManager`'s map of managed windows.
    fn start_managing_window(&mut self, window: core_graphics::Window) -> Result<()> {
        let window_id = window.number();
        let w = Window::try_from(window)?;
        let min_size = w.ax().min_size().unwrap_or_default();
        let cw = container::Window {
            id: window_id,
            min_width: min_size.width,
            min_height: min_size.height,
        };

        self.displays.add_window(cw)?;
        self.windows.insert(window_id, w);
        Ok(())
    }

    /// Update the information displayed on all managed status bars.
    ///
    /// This should be called whenever a change occurs regarding physical
    /// displays, logical displays, or global focus.
    fn update_status_bars(&mut self) {
        let active_id = self.displays.active_logical_display_id();
        for sb in self.status_bars.values_mut() {
            sb.draw(active_id);
        }
    }

    /// Move all windows to where they should be and set their size.
    ///
    /// Should be called whenever a window is added, removed, moved, toggled
    /// floating, minimised etc.
    fn apply_layout(&mut self) -> Result<()> {
        for pd in self.displays.physical_displays().values() {
            for (id, bounds) in pd.active_window_bounds() {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.update_bounds(bounds)?;
                    window.init()?;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let (keyboard_source, keyboard_sender) = ChannelSource::<KeyCommand>::new();
        let (workspace_source, workspace_sender) = ChannelSource::<WorkspaceEvent>::new();

        let keyboard_handler = KeyboardHandler::new(keyboard_sender).unwrap();
        let _workspace_observer = WorkspaceObserver::new(workspace_sender);
        let mut event_loop = EventLoop::new();

        let timer = Timer {
            id: 0,
            interval: Duration::from_secs(15),
        };

        // TODO: function
        let _ = std::fs::remove_file(CTL_SOCK);
        let ctl_sock = UnixListener::bind(CTL_SOCK).unwrap();

        let mux = Mux::new().unwrap();
        mux.add(&keyboard_source);
        mux.add(&workspace_source);
        mux.add(&timer);
        mux.add(&ctl_sock);

        unsafe {
            keyboard_handler
                .add_to_run_loop(CFRunLoopGetCurrent(), kCFRunLoopDefaultMode)
                .unwrap();
        }
        loop {
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) };

            let events = mux.poll();
            for event in events {
                match event {
                    Event::Readable(ident) if ident == keyboard_source.ident() => {
                        for command in keyboard_source.drain() {
                            self.handle_event(event_loop::Event::KeyCommand { command });
                        }
                    }
                    Event::Readable(ident) if ident == workspace_source.ident() => {
                        workspace_source.drain();
                        for event in event_loop.poll_windows() {
                            self.handle_event(event);
                        }
                    }
                    Event::Readable(ident) if ident == ctl_sock.ident() => {
                        println!("ctl sock rx");
                        let mut buf = Vec::with_capacity(20);
                        let (mut stream, _) = ctl_sock.accept().unwrap();
                        let _ = stream.read_to_end(&mut buf).unwrap();
                        println!("message on ctl sock: {buf:?}");
                        serde_json::to_writer(&mut stream, &WmToCtlMessage::Config(self.config))
                            .unwrap();
                        stream.shutdown(std::net::Shutdown::Write);
                    }
                    Event::Timer(ident) if ident == timer.ident() => {
                        println!("timer tick");
                    }
                    Event::Timer(ident) => {
                        println!("spurious timer event {ident}");
                    }
                    Event::Readable(ident) => {
                        println!("spurious read event {ident}")
                    }
                }
            }
        }
    }

    pub(super) fn handle_event(&mut self, event: event_loop::Event) {
        use event_loop::Event::*;

        match event {
            WindowAdded { display_id, window } => {
                ReceivedWindowAddedEvent(display_id, window.number()).log(&mut self.logger);

                if let Err(e) = self.handle_window_added(display_id, window) {
                    eprintln!("failed to add window: {e:?}");
                }
            }
            // TODO: Need to not trigger this from windows that disappear due to
            //       switching logical display
            WindowRemoved {
                display_id,
                window_id,
            } => {
                if self.minimised_windows.contains(&window_id) {
                    // MacOS registers a Core Graphics window removed event when
                    // the application is minimised. If it is a window we
                    // intentionally minimised, don't actually remove it.
                    return;
                }

                ReceivedWindowRemovedEvent(display_id, window_id).log(&mut self.logger);

                if let Err(e) = self.handle_window_removed(display_id, window_id) {
                    eprintln!("failed to remove window: {e:?}");
                }
            }
            KeyCommand { command } => {
                ReceivedKeyCommand(command).log(&mut self.logger);

                self.handle_key_command(command);
            }
            WindowFocused { window_id } => {
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
    fn handle_window_added(&mut self, id: DisplayId, cg: core_graphics::Window) -> Result<()> {
        let window_id = cg.number();
        let window = Window::try_from(cg)?;
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

                self.status_bars.get_mut(&id).unwrap().add_logical_id(lid);
                self.update_status_bars();
                return Ok(());
            }
        };

        self.status_bars.get_mut(&id).unwrap().add_logical_id(lid);
        self.windows.insert(window_id, window);
        self.update_status_bars();
        self.apply_layout()?;

        WindowAdded(lid, window_id).log(&mut self.logger);
        Ok(())
    }

    fn handle_window_removed(&mut self, display_id: DisplayId, window_id: WindowId) -> Result<()> {
        if self.floating_windows.contains(&window_id) {
            return Ok(());
        }

        self.displays.remove_window(display_id.into(), window_id)?;

        self.windows.remove(&window_id);
        self.apply_layout()?;

        WindowRemoved(self.displays.active_logical_display_id(), window_id).log(&mut self.logger);
        Ok(())
    }

    fn handle_key_command(&mut self, command: KeyCommand) {
        match command {
            KeyCommand::NewTerminal => {
                OpenTerminalKeyCommand.log(&mut self.logger);
                open_terminal();
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
            KeyCommand::ToggleFloating => {
                ToggleWindowFloatingKeyCommand.log(&mut self.logger);
                if let Err(e) = self.handle_toggle_floating() {
                    eprintln!("failed to toggle floating: {e:?}");
                }
            }
            _ => {}
        }
    }

    // To handle toggling a window to be floating:
    //  1. Get the currently focused window.
    //  2. If the currently focused window is already floating, add it to the
    //     active physical display, and mark it as not floating, else remove it
    //     from the active physical display, and mark it as floating.
    fn handle_toggle_floating(&mut self) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

        if self.floating_windows.contains(&focused_window) {
            self.floating_windows.remove(&focused_window);

            let cw = container::Window {
                id: focused_window,
                min_width: 0.0,
                min_height: 0.0,
            };

            self.displays.add_window(cw)?;
            self.apply_layout()?;

            WindowMadeManaged(focused_window).log(&mut self.logger);
        } else {
            let pid = self.displays.display_of_window(focused_window).unwrap();

            self.displays.remove_window(pid, focused_window)?;
            self.floating_windows.insert(focused_window);
            self.apply_layout()?;

            WindowMadeFloating(focused_window).log(&mut self.logger);
        }

        Ok(())
    }

    /// Handle a resize event in a given direction.
    fn handle_resize(&mut self, direction: Direction) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

        let active_display = self.active_physical_display_mut();
        active_display.set_focused_window(focused_window);
        active_display.resize_focused_window(direction)?;

        self.apply_layout()?;

        WindowResized(focused_window, direction).log(&mut self.logger);
        Ok(())
    }

    /// Handle splitting the currently focussed window's container along the
    /// provided axis.
    fn handle_split(&mut self, axis: container::Axis) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;
        self.displays.split(axis)?;
        self.apply_layout()?;

        WindowSplitAlongAxis(focused_window, axis).log(&mut self.logger);
        Ok(())
    }

    fn active_physical_display_mut(&mut self) -> &mut physical::Display {
        self.displays.active_physical_display_mut()
    }

    fn handle_focus_shift(&mut self, direction: Direction) -> Result<()> {
        let newly_focussed = self.active_physical_display_mut().shift_focus(direction)?;

        let window = self.windows.get_mut(&newly_focussed).unwrap();
        window.ax().try_focus().map_err(Error::AxUi)?;

        ShiftFocusInDirectionKeyCommand(direction).log(&mut self.logger);
        Ok(())
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
    fn handle_move_focused_window_to_display(&mut self, target: logical::Id) -> Result<()> {
        let focused_window = ax_ui::Window::try_get_focused().map_err(Error::AxUi)?;

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

        // Find the physical display that owns the currently focused window.
        let owner = self.displays.display_of_window(focused_window).unwrap();
        self.displays.remove_window(owner, focused_window)?;

        self.displays.add_window_to_logical(window, target)?;

        if let Some(focused) = self.displays.focus_display(target) {
            self.windows
                .get_mut(&focused)
                .unwrap()
                .ax()
                .try_focus()
                .map_err(Error::AxUi)?;
        }

        self.apply_layout()?;

        WindowMovedToLogicalDisplay(focused_window, target).log(&mut self.logger);
        Ok(())
    }

    /// Minimise all windows on the logical display referenced by the
    /// provided ID.
    fn try_minimise_logical(&mut self, id: logical::Id) -> Result<()> {
        for w in self.displays.get_logical(id).unwrap().window_ids() {
            self.minimised_windows.insert(w);
            self.windows
                .get_mut(&w)
                .unwrap()
                .ax()
                .minimise()
                .map_err(Error::AxUi)?;
        }

        Ok(())
    }

    /// Un-minimise all windows on the logical display referenced by the
    /// provided ID.
    fn try_unminimise_logical(&mut self, id: logical::Id) -> Result<()> {
        for w in self.displays.get_logical(id).unwrap().window_ids() {
            self.minimised_windows.insert(w);
            self.windows
                .get_mut(&w)
                .unwrap()
                .ax()
                .unminimise()
                .map_err(Error::AxUi)?;
        }

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
    fn handle_focus_logical_display(&mut self, new_lid: logical::Id) -> Result<()> {
        let current_lid = self.displays.active_logical_display_id();
        if current_lid == new_lid {
            return Ok(());
        }

        let current_pid = self.displays.logical_id_owner(current_lid).unwrap();
        let target_pid_exists = self.displays.logical_id_owner(new_lid);

        let target_pid = target_pid_exists.unwrap_or(current_pid);
        let same_pd = current_pid == target_pid;

        let to_minimise: logical::Id = if same_pd {
            current_lid
        } else {
            self.displays
                .physical_displays()
                .get(&target_pid)
                .unwrap()
                .active_logical_id()
        };
        self.try_minimise_logical(to_minimise)?;

        if target_pid_exists.is_none() {
            self.displays.create_logical_display(current_pid, new_lid);
        }

        self.displays.switch_logical_display(target_pid, new_lid);

        if same_pd
            && self
                .displays
                .get_logical(current_lid)
                .unwrap()
                .window_ids()
                .is_empty()
        {
            // Empty LD will already have been removed by DM
            self.status_bars
                .get_mut(&current_pid.into())
                .unwrap()
                .remove_logical_id(current_lid);
        }

        self.try_unminimise_logical(new_lid)?;

        if let Some(sb) = self.status_bars.get_mut(&target_pid.into()) {
            sb.add_logical_id(new_lid);
        }

        if let Some(focused) = self.displays.focus_display(new_lid) {
            self.windows
                .get_mut(&focused)
                .unwrap()
                .ax()
                .try_focus()
                .map_err(Error::AxUi)?;
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
        .arg("-F")
        .arg("-n")
        .arg("-a")
        .arg("Terminal")
        .spawn();

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
