mod bits;
mod error;
mod window;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use window::Window;
