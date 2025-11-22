mod bits;
mod label;
mod window;

use bits::{objc_getClass, objc_msgSend, sel_registerName};
use std::{ffi::CString, os::raw::c_void};

pub use label::Label;
pub use window::{Application, Window};

#[macro_export]
macro_rules! msg_send {
    ($obj:expr, $sel:expr) => {{
        let f: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { f($obj, $sel) }
    }};
    ($obj:expr, $sel:expr, $arg1:expr) => {{
        let f: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { f($obj, $sel, $arg1) }
    }};
    ($obj:expr, $sel:expr, $arg1:expr, $arg2:expr) => {{
        let f: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { f($obj, $sel, $arg1, $arg2) }
    }};
}

pub(crate) unsafe fn class(name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    objc_getClass(cname.as_ptr())
}

pub(crate) unsafe fn sel(name: &str) -> *mut c_void {
    let sname = CString::new(name).unwrap();
    sel_registerName(sname.as_ptr())
}

pub enum Colour {
    White,
    Black,
    Red,
    Green,
    Blue,
}

impl Colour {
    pub fn as_ns_colour(&self) -> *mut c_void {
        let colour_class = unsafe { class("NSColor") };

        let string = match self {
            Colour::White => "whiteColor",
            Colour::Black => "blackColor",
            Colour::Red => "redColor",
            Colour::Green => "greenColor",
            Colour::Blue => "blueColor",
        };

        msg_send!(colour_class, sel(string))
    }
}

pub trait NsElement {
    fn as_element(&self) -> *mut c_void;
}
