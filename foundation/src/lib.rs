use core_graphics::Bounds;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::ptr;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

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

unsafe fn class(name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    objc_getClass(cname.as_ptr())
}

unsafe fn sel(name: &str) -> *mut c_void {
    let sname = CString::new(name).unwrap();
    sel_registerName(sname.as_ptr())
}

pub struct Application {
    application: *mut c_void,
}

impl Application {
    pub unsafe fn new() -> Self {
        let application = class("NSApplication");
        let _shared = msg_send!(application, sel("sharedApplication"));

        Self { application }
    }
}

pub struct Window {
    window: *mut c_void,
}

impl Window {
    pub unsafe fn new(bounds: Bounds) -> Self {
        let window = class("NSWindow");
        let alloc = msg_send!(window, sel("alloc"));

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
        msg_send!(window, sel("setLevel:"), 25 as *mut c_void);

        Self { window }
    }

    pub unsafe fn set_background_colour(&mut self, colour: Colour) {
        msg_send!(
            self.window,
            sel("setBackgroundColor:"),
            colour.as_ns_colour()
        );
    }

    pub unsafe fn add_element_to_content_view<T: NsElement>(&mut self, element: T) {
        let content_view = msg_send!(self.window, sel("contentView"));
        msg_send!(content_view, sel("addSubview:"), element.as_element());
    }

    pub fn display(&self) {
        unsafe {
            msg_send!(self.window, sel("displayIfNeeded"));
            msg_send!(self.window, sel("makeKeyAndOrderFront:"), ptr::null_mut());
            msg_send!(self.window, sel("orderFrontRegardless"));
        }
    }

    pub unsafe fn close(&mut self) {
        msg_send!(self.window, sel("close"));
    }
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
            let text_alloc = msg_send!(text_class, sel("alloc"));

            let label_frame = [bounds.x, bounds.y, bounds.width, bounds.height];
            let init_label_fn: extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let label = init_label_fn(text_alloc, sel("initWithFrame:"), label_frame);
            msg_send!(label, sel("setBezeled:"), 0 as *mut c_void);
            msg_send!(label, sel("setDrawsBackground:"), 0 as *mut c_void);
            msg_send!(label, sel("setEditable:"), 0 as *mut c_void);
            msg_send!(label, sel("setSelectable:"), 0 as *mut c_void);

            msg_send!(label, sel("setTextColor:"), text_color.as_ns_colour());
            let nsstring = class("NSString");
            let ctext = CString::new(text).unwrap();
            let ns_str = msg_send!(
                nsstring,
                sel("stringWithUTF8String:"),
                ctext.as_ptr() as *mut c_void
            );
            msg_send!(label, sel("setStringValue:"), ns_str);

            Self { label }
        }
    }
}
