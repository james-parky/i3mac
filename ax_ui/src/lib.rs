mod bits;
mod error;
mod observer;
mod window;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use observer::Observer;
pub use observer::*;
pub use window::Window;

pub use bits::AXIsProcessTrusted;
