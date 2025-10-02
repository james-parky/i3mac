mod bits;
pub mod coregraphics;
mod error;
mod window;

pub use error::Error;
use std::result;
pub type Result<T> = result::Result<T, Error>;
