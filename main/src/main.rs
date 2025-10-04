use core_graphics::Display;

fn main() {
    match Display::all() {
        Ok(displays) => {
            println!("displays: {:?}", displays);
        }
        Err(err) => {
            eprintln!("could not get displays: {:?}", err);
        }
    }
}
