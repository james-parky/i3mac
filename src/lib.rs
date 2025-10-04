mod bits;
pub mod coregraphics;
mod dictionary;
pub mod display;
mod error;
pub mod window;

pub use error::Error;
use std::result;
pub type Result<T> = result::Result<T, Error>;
