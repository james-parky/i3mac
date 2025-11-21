use core_graphics::{DisplayId, KeyCommand, WindowId};
use std::collections::{HashMap, HashSet};

pub(super) enum Event {
    WindowAdded {
        display_id: DisplayId,
        window: core_graphics::Window,
    },
    WindowRemoved {
        display_id: DisplayId,
        window_id: WindowId,
    },
    WindowFocused {
        window_id: WindowId,
    },
    DisplayAdded {
        display_id: DisplayId,
        display: core_graphics::Display,
    },
    KeyCommand {
        command: KeyCommand,
    },
    // Tick,
}

pub(super) struct EventLoop {
    keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>,
    previous_displays: HashMap<DisplayId, HashSet<WindowId>>,
    managed_windows: HashSet<WindowId>,
}

impl EventLoop {
    pub(super) fn new(keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>) -> Self {
        Self {
            keyboard_rx,
            previous_displays: HashMap::new(),
            managed_windows: HashSet::new(),
        }
    }

    pub(super) fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        if let Ok(focused_window_id) = ax_ui::Window::try_get_focused() {
            events.push(Event::WindowFocused {
                window_id: focused_window_id,
            });
        }

        if let Ok(cg_displays) = core_graphics::Display::all() {
            for (display_id, cg_display) in cg_displays {
                let new_window_ids = cg_display.window_ids();

                match self.previous_displays.get(&display_id) {
                    None => {
                        events.push(Event::DisplayAdded {
                            display_id,
                            display: cg_display,
                        });
                        self.managed_windows.extend(&new_window_ids);
                        self.previous_displays.insert(display_id, new_window_ids);
                    }
                    Some(old_window_ids) => {
                        for &window_id in new_window_ids.difference(old_window_ids) {
                            if !self.managed_windows.contains(&window_id) {
                                if let Some(window) =
                                    cg_display.windows.iter().find(|w| w.number() == window_id)
                                {
                                    events.push(Event::WindowAdded {
                                        display_id,
                                        window: window.clone(),
                                    });
                                    self.managed_windows.insert(window_id);
                                }
                            }
                        }

                        for &window_id in old_window_ids.difference(&new_window_ids) {
                            events.push(Event::WindowRemoved {
                                display_id,
                                window_id,
                            });
                        }

                        self.previous_displays.insert(display_id, new_window_ids);
                    }
                }
            }
        }

        while let Ok(command) = self.keyboard_rx.try_recv() {
            events.push(Event::KeyCommand { command })
        }

        events
    }
}
