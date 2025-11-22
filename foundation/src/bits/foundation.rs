use std::os::raw::{c_char, c_void};

#[link(name = "AppKit", kind = "framework")]
// Clippy is wrong here. It thinks that there is duplication since both links
// are of type "framework", but we need them both.
#[allow(clippy::duplicated_attributes)]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    /// Get an Objective-C class by name.
    pub(crate) fn objc_getClass(name: *const c_char) -> *mut c_void;

    /// Register an Objective-C selector by name.
    pub(crate) fn sel_registerName(name: *const c_char) -> *mut c_void;

    /// Send a message to an Objective-C object.
    ///
    /// This is a variadic function that must be called through
    /// `std::mem::transmute()`.
    pub(crate) fn objc_msgSend();
}
