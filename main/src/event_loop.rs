// use core_graphics::{DisplayId, KeyCommand, WindowId};
// use std::collections::{HashMap, HashSet};
//
// #[derive(Debug)]
// pub(super) enum Event {
//     WindowAdded {
//         display_id: DisplayId,
//         window: core_graphics::Window,
//     },
//     WindowRemoved {
//         display_id: DisplayId,
//         window_id: WindowId,
//     },
//     WindowFocused {
//         window_id: WindowId,
//     },
//     #[allow(dead_code)]
//     DisplayAdded {
//         display_id: DisplayId,
//         display: core_graphics::Display,
//     },
//     KeyCommand {
//         command: KeyCommand,
//     },
// }
//
// pub(super) struct EventLoop {
//     keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>,
//     previous_displays: HashMap<DisplayId, HashSet<WindowId>>,
//     managed_windows: HashSet<WindowId>,
// }
//
// impl EventLoop {
//     pub(super) fn new(keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>) -> Self {
//         Self {
//             keyboard_rx,
//             previous_displays: HashMap::new(),
//             managed_windows: HashSet::new(),
//         }
//     }
//
//     pub(super) fn poll_keyboard(&mut self) -> Vec<Event> {
//         let mut events = Vec::new();
//         while let Ok(command) = self.keyboard_rx.try_recv() {
//             events.push(Event::KeyCommand { command })
//         }
//
//         events
//     }
//
//     pub(super) fn poll_windows(&mut self) -> Vec<Event> {
//         let mut events = Vec::new();
//         let mut current_windows = HashSet::<WindowId>::new();
//
//         if let Ok(cg_displays) = core_graphics::Display::all() {
//             for (display_id, cg_display) in cg_displays {
//                 let new_window_ids = cg_display.window_ids();
//
//                 println!("New window IDs: {:?} on {display_id}", new_window_ids);
//
//                 match self.previous_displays.get(&display_id) {
//                     None => {
//                         events.push(Event::DisplayAdded {
//                             display_id,
//                             display: cg_display,
//                         });
//                         self.managed_windows.extend(&new_window_ids);
//                         self.previous_displays.insert(display_id, new_window_ids);
//                     }
//                     Some(old_window_ids) => {
//                         println!("Old window IDs: {:?} on {display_id}", old_window_ids);
//                         for &window_id in new_window_ids.difference(old_window_ids) {
//                             println!("window {window_id} in new window ids but not old");
//                             if !self.managed_windows.contains(&window_id)
//                                 && let Some(window) =
//                                     cg_display.windows.iter().find(|w| w.number() == window_id)
//                             {
//                                 println!("creating window added event");
//                                 events.push(Event::WindowAdded {
//                                     display_id,
//                                     window: window.clone(),
//                                 });
//                                 self.managed_windows.insert(window_id);
//                             }
//                         }
//
//                         for &window_id in old_window_ids.difference(&new_window_ids) {
//                             events.push(Event::WindowRemoved {
//                                 display_id,
//                                 window_id,
//                             });
//                         }
//
//                         self.previous_displays.insert(display_id, new_window_ids);
//                     }
//                 }
//             }
//         }
//
//         if let Ok(focused_window_id) = ax_ui::Window::try_get_focused() {
//             events.push(Event::WindowFocused {
//                 window_id: focused_window_id,
//             });
//         }
//
//         events
//     }
// }
use core_graphics::{DisplayId, KeyCommand, WindowId};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
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
    #[allow(dead_code)]
    DisplayAdded {
        display_id: DisplayId,
        display: core_graphics::Display,
    },
    KeyCommand {
        command: KeyCommand,
    },
}

pub(super) struct EventLoop {
    keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>,
    previous_displays: HashMap<DisplayId, HashSet<WindowId>>,
    managed_windows: HashSet<WindowId>,
    /// Windows the WM has intentionally moved this loop iteration.
    /// poll_windows suppresses removal/addition events for these IDs and then
    /// clears the set. This handles the race where poll_windows runs in the
    /// same iteration as the keyboard event that triggered the move — at that
    /// point previous_displays is already updated, but the CG snapshot still
    /// reflects the pre-move state, so the diff would otherwise look like a
    /// real removal on the source display.
    recently_moved: HashSet<WindowId>,
}

impl EventLoop {
    pub(super) fn new(keyboard_rx: std::sync::mpsc::Receiver<KeyCommand>) -> Self {
        Self {
            keyboard_rx,
            previous_displays: HashMap::new(),
            managed_windows: HashSet::new(),
            recently_moved: HashSet::new(),
        }
    }

    pub(super) fn poll_keyboard(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(command) = self.keyboard_rx.try_recv() {
            events.push(Event::KeyCommand { command })
        }
        events
    }

    /// Notify the event loop that the WM has intentionally moved a window
    /// between physical displays.
    ///
    /// Two things happen:
    ///   1. `previous_displays` is updated so future polls see a consistent
    ///      baseline (covers the case where the move and the next poll are in
    ///      separate loop iterations).
    ///   2. The window is added to `recently_moved` so that the poll which runs
    ///      in the *same* loop iteration as the move — where CoreGraphics still
    ///      reflects the pre-move state — suppresses the false WindowRemoved on
    ///      the source display and the suppressed WindowAdded on the target.
    pub(super) fn notify_window_moved(
        &mut self,
        window_id: WindowId,
        from_display: DisplayId,
        to_display: DisplayId,
    ) {
        if let Some(ids) = self.previous_displays.get_mut(&from_display) {
            ids.remove(&window_id);
        }
        self.previous_displays
            .entry(to_display)
            .or_default()
            .insert(window_id);

        self.recently_moved.insert(window_id);
    }

    pub(super) fn poll_windows(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        // Drain the recently_moved set at the start of each poll. Any window
        // in here was moved by a WM command earlier this iteration — skip
        // emitting removal/addition events for it, and let previous_displays
        // (already updated by notify_window_moved) keep things consistent.
        let just_moved: HashSet<WindowId> = self.recently_moved.drain().collect();

        if let Ok(cg_displays) = core_graphics::Display::all() {
            for (display_id, cg_display) in cg_displays {
                let new_window_ids = cg_display.window_ids();

                println!("New window IDs: {:?} on {display_id}", new_window_ids);

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
                        println!("Old window IDs: {:?} on {display_id}", old_window_ids);

                        for &window_id in new_window_ids.difference(old_window_ids) {
                            // Skip windows that were just moved by the WM —
                            // their appearance here is expected, not a new window.
                            if just_moved.contains(&window_id) {
                                continue;
                            }
                            println!("window {window_id} in new window ids but not old");
                            if !self.managed_windows.contains(&window_id)
                                && let Some(window) =
                                    cg_display.windows.iter().find(|w| w.number() == window_id)
                            {
                                println!("creating window added event");
                                events.push(Event::WindowAdded {
                                    display_id,
                                    window: window.clone(),
                                });
                                self.managed_windows.insert(window_id);
                            }
                        }

                        for &window_id in old_window_ids.difference(&new_window_ids) {
                            // Skip windows that were just moved by the WM —
                            // their disappearance from this display is intentional.
                            if just_moved.contains(&window_id) {
                                continue;
                            }
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

        if let Ok(focused_window_id) = ax_ui::Window::try_get_focused() {
            events.push(Event::WindowFocused {
                window_id: focused_window_id,
            });
        }

        events
    }
}
