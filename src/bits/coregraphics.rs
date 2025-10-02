use std::ffi::{c_int, c_uint};

pub type CGDirectDisplayID = c_uint;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CGError(pub c_int);

impl CGError {
    pub const CANNOT_COMPILE: Self = Self(1004);
    pub const FAILURE: Self = Self(1000);
    pub const ILLEGAL_ARGUMENT: Self = Self(1001);
    pub const INVALID_CONNECTION: Self = Self(1002);
    pub const INVALID_CONTEXT: Self = Self(1003);
    pub const INVALID_OPERATION: Self = Self(1010);
    pub const NONE_AVAILABLE: Self = Self(1011);
    pub const NOT_IMPLEMENTED: Self = Self(1006);
    pub const RANGE_CHECK: Self = Self(1007);
    pub const SUCCESS: Self = Self(0);
    pub const TYPE_CHECK: Self = Self(1008);
}

pub type CGFloat = f64;

#[repr(C)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    // Documentation states this returns a CGError (int32_t) but it is better to
    // return a c_int here and cast it to the above _custom_ CGError to make it
    // easier to convert to Errors or Results.
    pub fn CGGetActiveDisplayList(
        max_displays: c_uint,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut c_uint,
    ) -> c_int;

    pub fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
}
