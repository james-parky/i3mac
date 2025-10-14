use crate::Error;
use core_foundation::{CFArrayRef, CFNumberType, CFTypeRef, cf_type_ref_to_num};
use std::ffi::{c_int, c_uint};
use std::ops::BitOr;

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

type CGFloat = f64;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
#[derive(Debug)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
#[derive(Debug)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    // Documentation states this returns a CGError (int32_t) but it is better to
    // return a c_int here and cast it to the above _custom_ CGError to make it
    // easier to convert to Errors or Results.
    pub fn CGGetActiveDisplayList(
        max_displays: c_uint,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut c_uint,
    ) -> c_int;

    pub fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;

    pub fn CGWindowListCopyWindowInfo(
        option: WindowListOption,
        relative_to_window: WindowId,
    ) -> CFArrayRef;
}

#[derive(Debug, Default)]
#[repr(u32)]
// Created as part of the Core Graphics ffi; yet are unused.
#[allow(dead_code)]
pub enum SharingType {
    #[default]
    None = 0,
    ReadOnly = 1,
    ReadWrite = 2,
}

#[derive(Debug, Default)]
#[repr(u32)]
// Created as part of the Core Graphics ffi; yet are unused.
#[allow(dead_code)]
pub enum StoreType {
    #[default]
    Retained = 0,
    NonRetained = 1,
    Buffered = 2,
}

impl TryFrom<CFTypeRef> for SharingType {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        // TODO: more specific error?
        cf_type_ref_to_num(value, CFNumberType::INT32).map_err(Error::CoreFoundation)
    }
}

impl TryFrom<CFTypeRef> for StoreType {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        // TODO: more specific error?
        cf_type_ref_to_num(value, CFNumberType::INT32).map_err(Error::CoreFoundation)
    }
}

// Core Graphics describes this as an enum, but Rust does not allow for BitOr
// between enum variants, so we use a new-type wrapper around a c_uint and
// provide constants for what the enum variants would have been.
#[repr(transparent)]
pub struct WindowListOption(c_uint);
impl WindowListOption {
    // pub const ALL: Self = Self(0);
    pub const ON_SCREEN_ONLY: Self = Self(1);
    // pub const ON_SCREEN_ABOVE_WINDOW: Self = Self(2);
    // pub const ON_SCREEN_BELOW_WINDOW: Self = Self(4);
    // pub const INCLUDING_WINDOW: Self = Self(8);
    pub const EXCLUDE_DESKTOP_ELEMENTS: Self = Self(16);
}

impl BitOr for WindowListOption {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self((self.0) | (rhs.0))
    }
}

#[repr(u32)]
pub enum WindowId {
    Null = 0,
}
