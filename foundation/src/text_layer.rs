use crate::{Colour, bits::objc_msgSend, class, msg_send, sel};
use core_graphics::Bounds;
use std::{ffi::CString, os::raw::c_void};

pub struct TextLayer {
    layer: *mut c_void,
}

impl TextLayer {
    pub fn new(bounds: Bounds, text: String, colour: Colour) -> Self {
        unsafe {
            let layer = msg_send!(class("CATextLayer"), sel("alloc"));
            let layer = msg_send!(layer, sel("init"));

            type SetFrameFn = unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]);
            let set_frame: SetFrameFn = std::mem::transmute(objc_msgSend as *const ());
            set_frame(
                layer,
                sel("setFrame:"),
                [bounds.x, bounds.y - 2.5, bounds.width, bounds.height],
            );

            let string_class = class("NSString");
            let c_text = CString::new(text).unwrap();
            let ns_str = msg_send!(
                string_class,
                sel("stringWithUTF8String:"),
                c_text.as_ptr() as *mut c_void
            );
            msg_send!(layer, sel("setString:"), ns_str);

            msg_send!(layer, sel("setForegroundColor:"), colour.as_cg_colour());

            let centre = {
                let ns_str_class = class("NSString");
                let c = CString::new("center").unwrap();
                msg_send!(
                    ns_str_class,
                    sel("stringWithUTF8String:"),
                    c.as_ptr() as *mut c_void
                )
            };
            msg_send!(layer, sel("setAlignmentMode:"), centre);

            msg_send!(layer, sel("setActions:"), std::ptr::null_mut::<c_void>());

            type SetFontSizeFn = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
            let set_size: SetFontSizeFn = std::mem::transmute(objc_msgSend as *const ());
            set_size(layer, sel("setFontSize:"), 12.0);

            type SetFontFn = unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void;
            let bold_font: SetFontFn = std::mem::transmute(objc_msgSend as *const ());
            let font = bold_font(class("NSFont"), sel("boldSystemFontOfSize:"), 12.0);
            msg_send!(layer, sel("setFont:"), font);

            type SetScaleFn = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
            let set_scale: SetScaleFn = std::mem::transmute(objc_msgSend as *const ());
            set_scale(layer, sel("setContentsScale:"), 2.0); // 2.0 for retina

            Self { layer }
        }
    }

    pub fn set_compositing_filter(&self, filter_name: &str) {
        unsafe {
            let string_class = class("NSString");
            let c_name = CString::new(filter_name).unwrap();
            let ns_str = msg_send!(
                string_class,
                sel("stringWithUTF8String:"),
                c_name.as_ptr() as *mut c_void
            );
            msg_send!(self.layer, sel("setCompositingFilter:"), ns_str);
        }
    }

    pub fn as_layer(&self) -> *mut c_void {
        self.layer
    }
}
