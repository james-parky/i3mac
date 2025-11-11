mod sys_info;

use crate::sys_info::{get_ipv4_address, get_ipv6_address};
use core_graphics::{Bounds, DisplayId};
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

pub struct StatusBar {
    window: Window,
}

struct Application {
    application: *mut c_void,
}

impl Application {
    pub unsafe fn new() -> Self {
        let application = class("NSApplication");
        let _shared = msg_send!(application, sel("sharedApplication"));

        Self { application }
    }
}

struct Window {
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
}

enum Colour {
    White,
    Black,
    Red,
    Green,
}

impl Colour {
    pub fn as_ns_colour(&self) -> *mut c_void {
        let colour_class = unsafe { class("NSColor") };

        let string = match self {
            Colour::White => "whiteColor",
            Colour::Black => "blackColor",
            Colour::Red => "redColor",
            Colour::Green => "greenColor",
        };

        msg_send!(colour_class, sel(string))
    }
}

impl StatusBar {
    pub fn new(display_id: DisplayId, bounds: Bounds) -> Self {
        unsafe {
            let _application = Application::new();

            let main_display_bounds = core_graphics::Display::main_display_bounds();
            let window_bottom_left = main_display_bounds.height - (bounds.y + bounds.height) - 25.0;
            let window_bounds = Bounds {
                x: bounds.x,
                y: window_bottom_left,
                height: 25.0,
                width: bounds.width,
            };
            let mut window = Window::new(window_bounds);

            println!("window bounds: {:?}", window_bounds);

            window.set_background_colour(Colour::Black);

            let ipv4_label_bounds = Bounds {
                x: bounds.width - 100.0,
                y: 0.0,
                height: 25.0,
                width: 100.0,
            };

            let ipv6_label_bounds = Bounds {
                x: bounds.width - 150.0,
                y: 0.0,
                height: 25.0,
                width: 50.0,
            };

            let display_id_bounds = Bounds {
                x: 0.0,
                y: 0.0,
                height: 25.0,
                width: 20.0,
            };

            window.add_element_to_content_view(Label::ipv4(ipv4_label_bounds));
            window.add_element_to_content_view(Label::ipv6(ipv6_label_bounds));
            window.add_element_to_content_view(Label::id(display_id, display_id_bounds));

            Self { window }
        }
    }

    pub fn display(&self) {
        self.window.display();
    }
}

pub trait NsElement {
    fn as_element(&self) -> *mut c_void;
}

struct Label {
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

    pub fn ipv4(bounds: Bounds) -> Self {
        let ipv4_addr = get_ipv4_address();
        let ipv4_addr_colour = if let Some(_) = ipv4_addr {
            Colour::Green
        } else {
            Colour::Red
        };

        unsafe {
            Label::new(
                bounds,
                ipv4_addr.unwrap_or("W: down".to_string()),
                ipv4_addr_colour,
            )
        }
    }

    pub fn ipv6(bounds: Bounds) -> Self {
        let ipv6_addr = get_ipv6_address();
        let ipv6_addr_colour = if let Some(_) = ipv6_addr {
            Colour::Green
        } else {
            Colour::Red
        };

        unsafe {
            Label::new(
                bounds,
                ipv6_addr.unwrap_or("no IPv6".to_string()),
                ipv6_addr_colour,
            )
        }
    }

    pub fn id(display_id: DisplayId, bounds: Bounds) -> Self {
        unsafe { Label::new(bounds, display_id.to_string(), Colour::White) }
    }
}
