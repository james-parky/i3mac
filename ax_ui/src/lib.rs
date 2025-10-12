mod bits;
mod error;
mod observer;
mod window;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use observer::Observer;
pub use window::Window;
