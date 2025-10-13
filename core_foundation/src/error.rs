use std::ffi::NulError;
use std::str::Utf8Error;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    NullCFArray,
    CannotCreateCString(NulError),
    NulString,
    InvalidCString(Utf8Error),
    NulDictionary,
}
