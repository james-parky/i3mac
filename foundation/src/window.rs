use crate::bits::objc_msgSend;
use crate::{Colour, NsElement, class, sel};
use core_graphics::Bounds;
use std::os::raw::c_void;

pub struct Application {
    #[allow(dead_code)]
    application: *mut c_void,
}

impl Default for Application {
    fn default() -> Self {
        unsafe {
            let application = class("NSApplication");
            let _shared = crate::msg_send!(application, sel("sharedApplication"));

            Self { application }
        }
    }
}

pub struct Window {
    window: *mut c_void,
}

impl Window {
    const NS_WINDOW_LEVEL_STATUS: i64 = 25;
    const NS_WINDOW_STYLE_MASK_BORDERLESS: u64 = 0;
    const NS_BACKING_STORE_BUFFERED: u64 = 2;

    pub fn new(bounds: Bounds) -> Self {
        unsafe {
            let window_class = class("NSWindow");
            let alloc = crate::msg_send!(window_class, sel("alloc"));
            let window = Self::init_window(alloc, bounds);
            Self::configure_window_level(window);

            Self { window }
        }
    }

    fn init_window(alloc: *mut c_void, bounds: Bounds) -> *mut c_void {
        type InitFn =
            unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4], u64, u64, bool) -> *mut c_void;

        let frame = [bounds.x, bounds.y, bounds.width, bounds.height];

        unsafe {
            let init_sel = sel("initWithContentRect:styleMask:backing:defer:");

            let init_fn: InitFn = std::mem::transmute(objc_msgSend as *const ());
            init_fn(
                alloc,
                init_sel,
                frame,
                Self::NS_WINDOW_STYLE_MASK_BORDERLESS,
                Self::NS_BACKING_STORE_BUFFERED,
                false,
            )
        }
    }

    fn configure_window_level(window: *mut c_void) {
        type SetLevelFn = unsafe extern "C" fn(*mut c_void, *mut c_void, i64);
        unsafe {
            let set_level: SetLevelFn = std::mem::transmute(objc_msgSend as *const ());
            set_level(window, sel("setLevel:"), Self::NS_WINDOW_LEVEL_STATUS);
        }
    }

    pub fn set_background_colour(&mut self, colour: Colour) {
        unsafe {
            crate::msg_send!(
                self.window,
                sel("setBackgroundColor:"),
                colour.as_ns_colour()
            );
        }
    }

    pub fn add_element_to_content_view<T: NsElement>(&mut self, element: T) {
        unsafe {
            let content_view = crate::msg_send!(self.window, sel("contentView"));
            crate::msg_send!(content_view, sel("addSubview:"), element.as_element());
        }
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

    pub fn close(&mut self) {
        unsafe {
            crate::msg_send!(self.window, sel("close"));
        }
    }
}
