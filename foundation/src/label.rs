use crate::{Colour, NsElement, bits::objc_msgSend, class, sel};
use core_graphics::Bounds;
use std::{ffi::CString, os::raw::c_void};

pub struct Label {
    label: *mut c_void,
}

impl NsElement for Label {
    fn as_element(&self) -> *mut c_void {
        self.label
    }
}

impl Label {
    #[must_use]
    pub fn new(bounds: Bounds, text: String, text_color: Colour) -> Self {
        let label = Self::create_text_field(bounds);
        Self::configure_label_appearance(label);
        Self::set_label_text_color(label, text_color);
        Self::set_label_text(label, text);

        Self { label }
    }

    #[must_use]
    fn create_text_field(bounds: Bounds) -> *mut c_void {
        let label_frame = [bounds.x, bounds.y, bounds.width, bounds.height];
        type InitFrameFunc =
            unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) -> *mut c_void;

        unsafe {
            let text_class = class("NSTextField");
            let text_alloc = crate::msg_send!(text_class, sel("alloc"));

            let init_label_fn: InitFrameFunc = std::mem::transmute(objc_msgSend as *const ());
            init_label_fn(text_alloc, sel("initWithFrame:"), label_frame)
        }
    }

    fn configure_label_appearance(label: *mut c_void) {
        type SetBoolFunc = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);

        unsafe {
            let set_bool: SetBoolFunc = std::mem::transmute(objc_msgSend as *const ());

            set_bool(label, sel("setBezeled:"), false);
            set_bool(label, sel("setDrawsBackground:"), false);
            set_bool(label, sel("setEditable:"), false);
            set_bool(label, sel("setSelectable:"), false);
        }
    }

    fn set_label_text_color(label: *mut c_void, text_color: Colour) {
        unsafe {
            crate::msg_send!(label, sel("setTextColor:"), text_color.as_ns_colour());
        }
    }

    fn set_label_text(label: *mut c_void, text: String) {
        unsafe {
            let string_class = class("NSString");
            let c_text = CString::new(text).expect("Invalid text string");

            let ns_str = crate::msg_send!(
                string_class,
                sel("stringWithUTF8String:"),
                c_text.as_ptr() as *mut c_void
            );
            crate::msg_send!(label, sel("setStringValue:"), ns_str);
        }
    }
}
