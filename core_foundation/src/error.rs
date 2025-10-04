use std::ffi::NulError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum Error {
    CannotComplete,
    Failure,
    IllegalArgument,
    InvalidConnection,
    InvalidContext,
    InvalidOperation,
    NoneAvailable,
    NotImplemented,
    RangeCheck,
    TypeCheck,
    NullActiveDisplay,
    NullCFArray,
    CannotCreateCString(NulError),
    NulString,
    InvalidCString(Utf8Error),
}
