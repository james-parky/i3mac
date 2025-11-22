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
    pub unsafe fn new(bounds: Bounds, text: String, text_color: Colour) -> Self {
        unsafe {
            let text_class = class("NSTextField");
            let text_alloc = crate::msg_send!(text_class, sel("alloc"));

            let label_frame = [bounds.x, bounds.y, bounds.width, bounds.height];
            let init_label_fn: extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let label = init_label_fn(text_alloc, sel("initWithFrame:"), label_frame);
            crate::msg_send!(label, sel("setBezeled:"), 0 as *mut c_void);
            crate::msg_send!(label, sel("setDrawsBackground:"), 0 as *mut c_void);
            crate::msg_send!(label, sel("setEditable:"), 0 as *mut c_void);
            crate::msg_send!(label, sel("setSelectable:"), 0 as *mut c_void);

            crate::msg_send!(label, sel("setTextColor:"), text_color.as_ns_colour());
            let nsstring = class("NSString");
            let ctext = CString::new(text).unwrap();
            let ns_str = crate::msg_send!(
                nsstring,
                sel("stringWithUTF8String:"),
                ctext.as_ptr() as *mut c_void
            );
            crate::msg_send!(label, sel("setStringValue:"), ns_str);

            Self { label }
        }
    }
}
