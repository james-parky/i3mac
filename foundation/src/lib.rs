pub mod bits;
mod id_label;
mod label;
mod text_layer;
mod window;

pub use id_label::IdLabel;
pub use label::Label;
pub use window::{Application, Window};

pub use bits::_NSConcreteStackBlock;
pub use bits::objc_msgSend;
use bits::{objc_getClass, sel_registerName};
use std::{ffi::CString, os::raw::c_void};

#[macro_export]
macro_rules! msg_send {
    ($obj:expr, $sel:expr) => {{
        $crate::msg_send_impl!($obj, $sel,)
    }};
    ($obj:expr, $sel:expr, $($arg:expr),+ $(,)?) => {{
        $crate::msg_send_impl!($obj, $sel, $($arg),+)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! msg_send_impl {
      ($obj:expr, $sel:expr, $($args:expr),*) => {{
        type MessageSendFunc = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void
            $(, $crate::msg_send_arg_type!($args))*
        ) -> *mut c_void;

        let f: MessageSendFunc = std::mem::transmute($crate::bits::objc_msgSend as *const ());

        f($obj, $sel $(, $args)*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! msg_send_arg_type {
    ($arg:expr) => {
        *mut c_void
    };
}

#[inline]
pub unsafe fn class(name: &str) -> *mut c_void {
    let class_name = CString::new(name).unwrap();
    unsafe { objc_getClass(class_name.as_ptr()) }
}

#[inline]
pub unsafe fn sel(name: &str) -> *mut c_void {
    let selector_name = CString::new(name).unwrap();
    unsafe { sel_registerName(selector_name.as_ptr()) }
}

pub enum Colour {
    White,
    Black,
    Red,
    Green,
    Blue,
    Clear,
}

impl Colour {
    pub fn as_ns_colour(&self) -> *mut c_void {
        unsafe {
            let colour_class = class("NSColor");
            let string = self.selector_name();
            msg_send!(colour_class, sel(string))
        }
    }

    pub fn as_cg_colour(&self) -> *mut c_void {
        unsafe { msg_send!(self.as_ns_colour(), sel("CGColor")) }
    }

    #[must_use]
    const fn selector_name(&self) -> &'static str {
        match self {
            Colour::White => "whiteColor",
            Colour::Black => "blackColor",
            Colour::Red => "redColor",
            Colour::Green => "greenColor",
            Colour::Blue => "blueColor",
            Colour::Clear => "clearColor",
        }
    }
}

pub trait NsElement {
    fn as_element(&self) -> *mut c_void;
}
