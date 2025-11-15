mod container;
mod display;
mod error;
mod event_loop;
mod window;
mod window_manager;

use crate::{event_loop::EventLoop, window_manager::WindowManager};
use core_foundation::{CFRunLoopGetCurrent, CFRunLoopRunInMode, kCFRunLoopDefaultMode};
use core_graphics::{KeyCommand, KeyboardHandler};
use std::sync::mpsc::channel;

fn main() {
    if !have_accessibility_permissions() {
        eprintln!("Accessibility permissions required!");
        return;
    }

    let (key_tx, key_rx) = channel::<KeyCommand>();
    let keyboard = KeyboardHandler::new(key_tx).expect("failed to create keyboard handler");

    unsafe { keyboard.add_to_run_loop(CFRunLoopGetCurrent(), kCFRunLoopDefaultMode) }
        .expect("failed to add keyboard to run loop");

    let mut wm = WindowManager::new();
    let mut event_loop = EventLoop::new(key_rx);

    loop {
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, false) }

        for event in event_loop.poll() {
            wm.handle_event(event);
        }
    }
}

fn have_accessibility_permissions() -> bool {
    unsafe { ax_ui::AXIsProcessTrusted() }
}
