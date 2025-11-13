use crate::{bits, bits::AxUiElementRef};
use std::ffi::NulError;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    ApiDisable,
    ActionUnsupported,
    AttributeUnsupported,
    CannotComplete,
    Failure,
    IllegalArgument,
    InvalidUiElement,
    InvalidUiElementObserver,
    NoValue,
    NotEnoughPrecision,
    NotImplemented,
    NotificationAlreadyRegistered,
    NotificationNotRegistered,
    NotificationUnsupported,
    ParameterisedAttributeUnsupported,
    UnknownAXError(bits::AXError),
    CannotMakeCString(NulError),
    PidDoesNotExist(libc::pid_t),
    IncorrectPrivilegesForPid(libc::pid_t),
    Unknown,
    CoreFoundation(core_foundation::Error),
    CouldNotFindWindow(libc::pid_t),
    CouldNotCreateObserver(libc::pid_t, bits::AXError),
    CouldNotAttachNotification(AxUiElementRef, bits::AXError),
    CouldNotRemoveNotification(AxUiElementRef, bits::AXError),
    CouldNotGetWindowNumber(AxUiElementRef),
    CouldNotGetWindowSize(AxUiElementRef),
    CouldNotGetFocusedWindow,
    CouldNotFocusWindow(AxUiElementRef),
    CouldNotGetPid,
}

impl From<bits::AXError> for Option<Error> {
    fn from(value: bits::AXError) -> Self {
        match value {
            bits::AXError::API_DISABLE => Some(Error::ApiDisable),
            bits::AXError::ACTION_UNSUPPORTED => Some(Error::ActionUnsupported),
            bits::AXError::ATTRIBUTE_UNSUPPORTED => Some(Error::AttributeUnsupported),
            bits::AXError::CANNOT_COMPLETE => Some(Error::CannotComplete),
            bits::AXError::FAILURE => Some(Error::Failure),
            bits::AXError::ILLEGAL_ARGUMENT => Some(Error::IllegalArgument),
            bits::AXError::INVALID_UI_ELEMENT => Some(Error::InvalidUiElement),
            bits::AXError::INVALID_UI_ELEMENT_OBSERVER => Some(Error::InvalidUiElementObserver),
            bits::AXError::NO_VALUE => Some(Error::NoValue),
            bits::AXError::NOT_ENOUGH_PRECISION => Some(Error::NotEnoughPrecision),
            bits::AXError::NOT_IMPLEMENTED => Some(Error::NotImplemented),
            bits::AXError::NOTIFICATION_ALREADY_REGISTERED => {
                Some(Error::NotificationAlreadyRegistered)
            }
            bits::AXError::NOTIFICATION_NOT_REGISTERED => Some(Error::NotificationNotRegistered),
            bits::AXError::NOTIFICATION_UNSUPPORTED => Some(Error::NotificationUnsupported),
            bits::AXError::PARAMETERISED_ATTRIBUTE_UNSUPPORTED => {
                Some(Error::ParameterisedAttributeUnsupported)
            }
            bits::AXError::SUCCESS => None,
            x => Some(Error::UnknownAXError(x)),
        }
    }
}
