mod array;
mod bits;
mod dictionary;
mod error;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub use array::*;
pub use bits::*;
pub use dictionary::*;
