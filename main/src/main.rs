mod container;
mod display;
mod error;
mod event_loop;
mod log;
mod status_bar;
mod sys_info;
mod window;
mod window_manager;

use crate::{event_loop::EventLoop, window_manager::Config, window_manager::WindowManager};
use core_foundation::{CFRunLoopGetCurrent, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::{KeyCommand, KeyboardHandler};
use foundation::{WorkspaceEvent, WorkspaceObserver};
use std::sync::mpsc::channel;

fn main() {
    if !have_accessibility_permissions() {
        eprintln!("Accessibility permissions required!");
        return;
    }

    let (key_tx, key_rx) = channel::<KeyCommand>();
    let keyboard = KeyboardHandler::new(key_tx).expect("failed to create keyboard handler");

    // Safety:
    //  - The `run_loop` supplied to `KeyboardHandler::add_run_loop()` is valid
    //    as it was returned by the library function `CFRunLoopGetCurrent()`.
    unsafe { keyboard.add_to_run_loop(CFRunLoopGetCurrent(), kCFRunLoopDefaultMode) }
        .expect("failed to add keyboard to run loop");

    let (workspace_tx, workspace_rx) = channel::<WorkspaceEvent>();
    let _workspace_observer = WorkspaceObserver::new(workspace_tx);

    let cfg = Config::must_parse();
    let mut wm = WindowManager::new(cfg);
    let mut event_loop = EventLoop::new(key_rx);

    loop {
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) }

        let mut should_poll_windows = false;
        if let Ok(_event) = workspace_rx.try_recv() {
            should_poll_windows = true;
        }

        if should_poll_windows {
            for event in event_loop.poll_windows() {
                wm.handle_event(event);
            }
        }

        wm.reset_windows();

        for event in event_loop.poll_keyboard() {
            wm.handle_event(event);
        }
    }
}

fn have_accessibility_permissions() -> bool {
    unsafe { ax_ui::AXIsProcessTrusted() }
}
