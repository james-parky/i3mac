use core_foundation::CFRunLoopRun;
use core_graphics::Display;

fn main() {
    match Display::all() {
        Ok(displays) => {
            for display in displays.values() {
                for window in &display.windows {
                    let ax_window =
                        ax_ui::Window::new(window.owner_pid(), window.bounds()).unwrap();
                    println!("{:?}", ax_window);
                    // println!("{:?}", ax_window.move_to(-100.00, 0.0));
                }
            }
        }
        Err(err) => {
            eprintln!("could not get displays: {:?}", err);
            return;
        }
    }

    unsafe { CFRunLoopRun() };
}
