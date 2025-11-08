mod container;
mod display;
mod window;

use crate::display::Display;
use core_foundation::{CFRunLoopGetCurrent, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::{Direction, DisplayId, KeyCommand, KeyboardHandler, WindowId};
use std::{collections::HashMap, hash::Hash, sync::mpsc::channel};

#[derive(Debug, Eq, PartialEq)]
enum Error {
    AxUi(ax_ui::Error),
    CoreGraphics(core_graphics::Error),
    CGWindowMissingName(String),
    WindowNotFound,
    DisplayNotFound,
    NoWindowsOnDisplay,
    CannotAddWindowToLeaf,
    CannotSplitEmptyContainer,
    CannotSplitAlreadySplitContainer,
}

struct Context {
    displays: HashMap<DisplayId, Display>,
}

type Result<T> = std::result::Result<T, Error>;

fn main() {
    if !have_accessibility_permissions() {
        eprintln!("Accessibility permissions required!");
        return;
    }

    let (key_tx, key_rx) = channel::<KeyCommand>();
    let keyboard = KeyboardHandler::new(key_tx).expect("failed to create keyboard handler");

    unsafe { keyboard.add_to_run_loop(CFRunLoopGetCurrent(), kCFRunLoopDefaultMode) };

    let mut ctx = Context {
        displays: HashMap::new(),
    };

    loop {
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) }

        if let Ok(focused_window_id) = ax_ui::Window::get_focused() {
            for display in ctx.displays.values_mut() {
                if display.window_ids().contains(&focused_window_id) {
                    display.set_focused_window(focused_window_id);
                    break;
                }
            }
        }

        match core_graphics::Display::all() {
            Ok(cg_displays_map) => {
                for (display_id, cg_display) in cg_displays_map {
                    let current_ids = ctx
                        .displays
                        .get(&display_id)
                        .map(|d| d.window_ids())
                        .unwrap_or_default();
                    let new_ids = cg_display.window_ids();

                    if current_ids == new_ids {
                        continue;
                    }

                    if !ctx.displays.contains_key(&display_id) {
                        let mut display = Display::new(cg_display.bounds);
                        for cg_window in cg_display.windows {
                            display
                                .add_window(cg_window)
                                .expect("failed to insert focused window");
                        }
                        ctx.displays.insert(display_id, display);
                        continue;
                    }

                    let removed: Vec<_> = current_ids.difference(&new_ids).collect();
                    if !removed.is_empty() {
                        for window_id in removed {
                            if let Some(display) = ctx.displays.get_mut(&display_id) {
                                match display.remove_window(*window_id) {
                                    Err(e) => println!("error removing: {:?}", e),
                                    _ => {}
                                }
                            }
                        }
                    }

                    let added: Vec<_> = new_ids.difference(&current_ids).collect();
                    if !added.is_empty() {
                        for window_id in added {
                            if let Some(cg_window) =
                                cg_display.windows.iter().find(|w| w.number() == *window_id)
                            {
                                if let Some(display) = ctx.displays.get_mut(&display_id) {
                                    match display.add_window(cg_window.clone()) {
                                        Err(e) => println!("error adding: {:?}", e),
                                        Ok(_) => {
                                            display.set_focused_window(*window_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                println!("POLL ERROR: {err:?}")
            }
        }

        while let Ok(command) = key_rx.try_recv() {
            handle_key_command(command, &mut ctx);
        }
    }
}

fn handle_key_command(command: KeyCommand, ctx: &mut Context) {
    match command {
        KeyCommand::NewTerminal => {
            open_terminal();
        }
        KeyCommand::CloseWindow => {
            // if let Err(e) = close_focused_window(&mut ctx.displays) {
            //     println!("Failed to close window: {:?}", e);
            // }
        }
        KeyCommand::Focus(Direction::Left)
        | KeyCommand::Focus(Direction::Right)
        | KeyCommand::Focus(Direction::Up)
        | KeyCommand::Focus(Direction::Down) => {
            if let Err(e) = handle_focus_shift(command, &mut ctx.displays) {
                println!("Failed to shift focus: {:?}", e);
            }
        }
        KeyCommand::FocusDisplay(n) => {
            if let Err(e) = focus_display(n as u8, &mut ctx.displays) {
                println!("Failed to focus display: {:?}", e);
            }
        }
        KeyCommand::MoveWindowToDisplay(n) => {
            // if let Err(e) = move_window_to_display(n as u8, displays) {
            //     println!("Failed to move window: {:?}", e);
            // }
        }
        KeyCommand::MoveWindow(_direction) => {
            // if let Ok((_, window_id)) = ax_ui::Window::get_focused() {
            //     for display in displays.values_mut() {
            //         if display.move_window(window_id, direction) {
            //             break;
            //         }
            //     }
            // }
        }
        KeyCommand::ToggleVerticalSplit => {
            handle_split(&mut ctx.displays, container::Direction::Vertical);
        }
        KeyCommand::ToggleHorizontalSplit => {
            handle_split(&mut ctx.displays, container::Direction::Horizontal);
        }
    }
}

fn handle_split(displays: &mut HashMap<DisplayId, Display>, direction: container::Direction) {
    if let Ok(window_id) = ax_ui::Window::get_focused() {
        for display in displays.values_mut() {
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

fn open_terminal() {
    use std::process::Command;
    let _ = Command::new("open")
        .arg("-n")
        .arg("-a")
        .arg("Terminal")
        .spawn();
}

fn handle_focus_shift(command: KeyCommand, displays: &HashMap<DisplayId, Display>) -> Result<()> {
    let current_window_id = ax_ui::Window::get_focused().map_err(Error::AxUi)?;

    // Find which display has the focused window
    for (display_id, display) in displays.iter() {
        // let windows = display.get_windows_ordered();
        let unordered_windows = display.window_ids();

        let windows: Vec<WindowId> = unordered_windows.into_iter().collect();
        println!("Window {:?}", windows);

        if let Some(current_idx) = windows.iter().position(|&id| id == current_window_id) {
            // Found the display with this window
            let next_idx = match command {
                KeyCommand::Focus(Direction::Left) | KeyCommand::Focus(Direction::Up) => {
                    if current_idx == 0 {
                        windows.len() - 1
                    } else {
                        current_idx - 1
                    }
                }
                KeyCommand::Focus(Direction::Right) | KeyCommand::Focus(Direction::Down) => {
                    (current_idx + 1) % windows.len()
                }
                _ => return Ok(()),
            };

            let next_window_id = windows[next_idx];
            println!(
                "current: {:?}, next: {:?}",
                current_window_id, next_window_id
            );
            return display.focus_window(next_window_id);
        }
    }

    Err(Error::WindowNotFound)
}

fn focus_display(display_num: u8, displays: &HashMap<DisplayId, Display>) -> Result<()> {
    let display = displays
        .get(&(display_num as usize).into())
        .ok_or(Error::DisplayNotFound)?;

    if let Some(&first_window) = display.window_ids().iter().nth(0) {
        display.focus_window(first_window.into())
    } else {
        Err(Error::NoWindowsOnDisplay)
    }
}

fn have_accessibility_permissions() -> bool {
    unsafe { ax_ui::AXIsProcessTrusted() }
}
