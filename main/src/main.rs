mod container;
mod display;
mod error;
mod event_loop;
mod log;
mod status_bar;
mod sys_info;
mod window;
mod window_manager;

use crate::{window_manager::Config, window_manager::WindowManager};

fn main() {
    if !have_accessibility_permissions() {
        eprintln!("Accessibility permissions required!");
        return;
    }

    let cfg = Config::must_parse();
    let mut wm = WindowManager::new(cfg).expect("failed to create window manager");

    if let Err(e) = wm.run() {
        eprintln!("Window Manager exited: {e:?}");
    }
}

fn have_accessibility_permissions() -> bool {
    unsafe { ax_ui::AXIsProcessTrusted() }
}
