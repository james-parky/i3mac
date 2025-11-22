use core_foundation::{
    CFRunLoopSourceRef, CFStringRef, CFTypeRef, kCFBooleanFalse, kCFBooleanTrue,
};
use core_graphics::CGSize;
use std::cmp::PartialEq;
use std::ffi::{c_int, c_uint, c_void};

pub type AxUiElementRef = *const c_void;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct AXError(pub c_int);

impl AXError {
    pub const API_DISABLE: Self = Self(-25211);
    pub const ACTION_UNSUPPORTED: Self = Self(-25206);
    pub const ATTRIBUTE_UNSUPPORTED: Self = Self(-25205);
    pub const CANNOT_COMPLETE: Self = Self(-25204);
    pub const FAILURE: Self = Self(-25200);
    pub const ILLEGAL_ARGUMENT: Self = Self(-25201);
    pub const INVALID_UI_ELEMENT: Self = Self(-25202);
    pub const INVALID_UI_ELEMENT_OBSERVER: Self = Self(-25203);
    pub const NO_VALUE: Self = Self(-25212);
    pub const NOT_ENOUGH_PRECISION: Self = Self(-25214);
    pub const NOT_IMPLEMENTED: Self = Self(-25208);
    pub const NOTIFICATION_ALREADY_REGISTERED: Self = Self(-25209);
    pub const NOTIFICATION_NOT_REGISTERED: Self = Self(-25210);
    pub const NOTIFICATION_UNSUPPORTED: Self = Self(-25207);
    pub const PARAMETERISED_ATTRIBUTE_UNSUPPORTED: Self = Self(-25213);
    pub const SUCCESS: Self = Self(0);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct AXValueType(pub c_uint);

// TODO: should unused constants only implemented to mimic the library be removed?
impl AXValueType {
    #[allow(dead_code)]
    pub const AX_ERROR: Self = Self(5);
    #[allow(dead_code)]
    pub const CF_RANGE: Self = Self(4);
    #[allow(dead_code)]
    pub const CG_POINT: Self = Self(1);
    #[allow(dead_code)]
    pub const CG_RECT: Self = Self(3);
    pub const CG_SIZE: Self = Self(2);
    #[allow(dead_code)]
    pub const ILLEGAL: Self = Self(0);
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXUIElementCreateApplication(pid: libc::pid_t) -> AxUiElementRef;
    pub fn AXUIElementSetAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: *const c_void,
    ) -> c_int;

    pub fn AXUIElementPerformAction(element: AxUiElementRef, action: CFStringRef) -> c_int;

    pub fn AXUIElementCopyAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: *mut *const c_void,
    ) -> c_int;

    pub fn AXValueCreate(type_: AXValueType, value: *const c_void) -> AXValueRef;

    pub fn AXValueGetValue(value_ref: AXValueRef, type_: AXValueType, value: *mut c_void) -> bool;

    pub fn AXObserverCreate(
        application: libc::pid_t,
        callback: AXObserverCallback,
        observer: &mut AXObserverRef,
    ) -> c_int;

    pub fn AXObserverAddNotification(
        observer: AXObserverRef,
        element: AxUiElementRef,
        notification: CFStringRef,
        context: *mut c_void,
    ) -> c_int;

    pub fn AXObserverRemoveNotification(
        observer: AXObserverRef,
        element: AxUiElementRef,
        notification: CFStringRef,
    ) -> c_int;

    pub fn AXObserverGetRunLoopSource(observer: AXObserverRef) -> CFRunLoopSourceRef;

    pub fn _AXUIElementGetWindow(
        element: CFTypeRef,
        out_window_id: *mut core_graphics::WindowId,
    ) -> c_int;

    pub fn AXUIElementCreateSystemWide() -> AxUiElementRef;

    pub fn AXIsProcessTrusted() -> bool;
}

pub type AXObserverRef = *const c_void;

pub type AXObserverCallback = extern "C" fn(
    observer: AXObserverRef,
    element: AxUiElementRef,
    notification: CFStringRef,
    ref_con: *mut c_void,
);

#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct AXValueRef(pub *const c_void);

impl TryFrom<AXValueRef> for CGSize {
    type Error = crate::Error;

    fn try_from(ax_value: AXValueRef) -> Result<Self, Self::Error> {
        let mut size = CGSize {
            width: 0.0,
            height: 0.0,
        };

        let success = unsafe {
            AXValueGetValue(
                ax_value,
                AXValueType::CG_SIZE,
                &mut size as *mut _ as *mut c_void,
            )
        };

        if success {
            Ok(size)
        } else {
            Err(Self::Error::CouldNotExtractValue)
        }
    }
}

impl TryFrom<AXValueRef> for bool {
    type Error = crate::Error;
    fn try_from(ax_value: AXValueRef) -> Result<Self, Self::Error> {
        if ax_value.0 == unsafe { kCFBooleanTrue } {
            Ok(true)
        } else if ax_value.0 == unsafe { kCFBooleanFalse } {
            Ok(false)
        } else {
            Err(Self::Error::CouldNotExtractValue)
        }
    }
}
