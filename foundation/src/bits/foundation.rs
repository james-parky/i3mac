use std::os::raw::{c_char, c_void};

#[link(name = "AppKit", kind = "framework")]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    pub(crate) fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub(crate) fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub(crate) fn objc_msgSend();
}
