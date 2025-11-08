use crate::bits;
use std::ffi::NulError;
use std::str::Utf8Error;

#[derive(Debug, Eq, PartialEq)]
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
    UnknownCGError(bits::CGError),
    NullActiveDisplay,
    NullCFArray,
    CannotCreateCString(NulError),
    NulString,
    InvalidCString(Utf8Error),
    CoreFoundation(core_foundation::Error),
    CouldNotFindDictionaryKey(String),
    FailedToCreateKeyboardEventTap,
}

impl From<bits::CGError> for Option<Error> {
    fn from(value: bits::CGError) -> Self {
        match value {
            bits::CGError::CANNOT_COMPILE => Some(Error::CannotComplete),
            bits::CGError::FAILURE => Some(Error::Failure),
            bits::CGError::ILLEGAL_ARGUMENT => Some(Error::IllegalArgument),
            bits::CGError::INVALID_CONNECTION => Some(Error::InvalidConnection),
            bits::CGError::INVALID_CONTEXT => Some(Error::InvalidContext),
            bits::CGError::INVALID_OPERATION => Some(Error::InvalidOperation),
            bits::CGError::NONE_AVAILABLE => Some(Error::NoneAvailable),
            bits::CGError::NOT_IMPLEMENTED => Some(Error::NotImplemented),
            bits::CGError::RANGE_CHECK => Some(Error::RangeCheck),
            bits::CGError::TYPE_CHECK => Some(Error::TypeCheck),
            bits::CGError::SUCCESS => None,
            x => Some(Error::UnknownCGError(x)),
        }
    }
}
