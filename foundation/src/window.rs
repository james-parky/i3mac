use crate::bits::objc_msgSend;
use crate::{Colour, NsElement, class, sel};
use core_graphics::Bounds;
use std::os::raw::c_void;

pub struct Application {
    application: *mut c_void,
}

impl Application {
    pub unsafe fn new() -> Self {
        let application = class("NSApplication");
        let _shared = crate::msg_send!(application, sel("sharedApplication"));

        Self { application }
    }
}

pub struct Window {
    window: *mut c_void,
}

impl Window {
    pub unsafe fn new(bounds: Bounds) -> Self {
        let window = class("NSWindow");
        let alloc = crate::msg_send!(window, sel("alloc"));

        let init_sel = sel("initWithContentRect:styleMask:backing:defer:");
        let frame = [bounds.x, bounds.y, bounds.width, bounds.height];
        let style_mask = 0u64;
        let backing = 2u64;
        let defer = false;

        let init_fn: extern "C" fn(
            *mut c_void,
            *mut c_void,
            [f64; 4],
            u64,
            u64,
            bool,
            bool,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());

        let window = init_fn(alloc, init_sel, frame, style_mask, backing, defer, true);

        // TODO: can this be done in init_fn?
        crate::msg_send!(window, sel("setLevel:"), 25 as *mut c_void);

        Self { window }
    }

    pub unsafe fn set_background_colour(&mut self, colour: Colour) {
        crate::msg_send!(
            self.window,
            sel("setBackgroundColor:"),
            colour.as_ns_colour()
        );
    }

    pub unsafe fn add_element_to_content_view<T: NsElement>(&mut self, element: T) {
        let content_view = crate::msg_send!(self.window, sel("contentView"));
        crate::msg_send!(content_view, sel("addSubview:"), element.as_element());
    }

    pub fn display(&self) {
        unsafe {
            crate::msg_send!(self.window, sel("displayIfNeeded"));
            crate::msg_send!(
                self.window,
                sel("makeKeyAndOrderFront:"),
                std::ptr::null_mut()
            );
            crate::msg_send!(self.window, sel("orderFrontRegardless"));
        }
    }

    pub unsafe fn close(&mut self) {
        crate::msg_send!(self.window, sel("close"));
    }
}
