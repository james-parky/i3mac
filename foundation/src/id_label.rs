use crate::{Colour, NsElement, bits::objc_msgSend, class, msg_send, sel, text_layer::TextLayer};
use core_graphics::Bounds;
use std::os::raw::c_void;

pub struct IdLabel {
    view: *mut c_void,
}

impl NsElement for IdLabel {
    fn as_element(&self) -> *mut c_void {
        self.view
    }
}

impl IdLabel {
    pub fn new_active(bounds: Bounds, text: String) -> Self {
        unsafe {
            let view = Self::make_view(bounds);

            msg_send!(view, sel("setWantsLayer:"), 1usize as *mut c_void);
            let root_layer = msg_send!(view, sel("layer"));

            // White filled background
            let bg_layer = Self::make_layer(bounds);
            msg_send!(
                bg_layer,
                sel("setBackgroundColor:"),
                Colour::White.as_cg_colour()
            );

            // Text layer with destinationOut punches through the white
            let text_bounds = Bounds {
                x: 0.0,
                y: 3.0,
                ..bounds
            }; // y offset to vertically centre in layer
            let text_layer = TextLayer::new(
                Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
                text,
                Colour::Black, // colour doesn't matter, destinationOut uses alpha
            );
            text_layer.set_compositing_filter("destinationOut");

            msg_send!(root_layer, sel("addSublayer:"), bg_layer);
            msg_send!(bg_layer, sel("addSublayer:"), text_layer.as_layer());

            // The layer needs to allow transparency for destinationOut to work
            msg_send!(root_layer, sel("setMasksToBounds:"), 1usize as *mut c_void);

            Self { view }
        }
    }

    pub fn new_inactive(bounds: Bounds, text: String) -> Self {
        unsafe {
            let view = Self::make_view(bounds);

            msg_send!(view, sel("setWantsLayer:"), 1usize as *mut c_void);
            let root_layer = msg_send!(view, sel("layer"));

            // Transparent background with white border
            let bg_layer = Self::make_layer(bounds);
            msg_send!(
                bg_layer,
                sel("setBackgroundColor:"),
                Colour::Clear.as_cg_colour()
            );
            msg_send!(
                bg_layer,
                sel("setBorderColor:"),
                Colour::White.as_cg_colour()
            );

            type SetWidthFn = unsafe extern "C" fn(*mut c_void, *mut c_void, f64);
            let set_width: SetWidthFn = std::mem::transmute(objc_msgSend as *const ());
            set_width(bg_layer, sel("setBorderWidth:"), 2.0);

            // Plain white text, no compositing trick needed
            let text_layer = TextLayer::new(
                Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
                text,
                Colour::White,
            );

            msg_send!(root_layer, sel("addSublayer:"), bg_layer);
            msg_send!(bg_layer, sel("addSublayer:"), text_layer.as_layer());

            Self { view }
        }
    }

    unsafe fn make_view(bounds: Bounds) -> *mut c_void {
        let view_class = class("NSView");
        let alloc = msg_send!(view_class, sel("alloc"));

        type InitFn = unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) -> *mut c_void;
        let init_fn: InitFn = std::mem::transmute(objc_msgSend as *const ());
        init_fn(
            alloc,
            sel("initWithFrame:"),
            [bounds.x, bounds.y, bounds.width, bounds.height],
        )
    }

    unsafe fn make_layer(bounds: Bounds) -> *mut c_void {
        let layer = msg_send!(class("CALayer"), sel("alloc"));
        let layer = msg_send!(layer, sel("init"));

        type SetFrameFn = unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]);
        let set_frame: SetFrameFn = std::mem::transmute(objc_msgSend as *const ());
        set_frame(
            layer,
            sel("setFrame:"),
            [0.0, 0.0, bounds.width, bounds.height],
        );

        layer
    }
}
