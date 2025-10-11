use core_foundation::{CFArrayRef, CFRunLoopSourceRef, CFStringRef};
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

impl AXValueType {
    pub const AX_ERROR: Self = Self(5);
    pub const CF_RANGE: Self = Self(4);
    pub const CG_POINT: Self = Self(1);
    pub const CG_RECT: Self = Self(3);
    pub const CG_SIZE: Self = Self(2);
    pub const ILLEGAL: Self = Self(0);
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXUIElementCreateApplication(pid: libc::pid_t) -> AxUiElementRef;
    // Documentation states this returns a AXError (int32_t) but it is better to
    // return a c_int here and cast it to the above _custom_ AXError to make it
    // easier to convert to Errors or Results.
    pub fn AXUIElementCopyAttributeNames(element: AxUiElementRef, names: &mut CFArrayRef) -> c_int;
    // Documentation states this returns a AXError (int32_t) but it is better to
    // return a c_int here and cast it to the above _custom_ AXError to make it
    // easier to convert to Errors or Results.
    fn AXUIElementCopyActionNames(element: AxUiElementRef, names: &mut CFArrayRef) -> c_int;
    fn AXUIElementIsAttributeSettable(
        element: AxUiElementRef,
        attribute: CFStringRef,
        settable: *mut bool,
    ) -> c_int;

    pub fn AXUIElementSetAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: *const c_void,
    ) -> c_int;

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
}

pub type AXObserverRef = *const c_void;

pub type AXObserverCallback = extern "C" fn(
    observer: AXObserverRef,
    element: AxUiElementRef,
    notification: CFStringRef,
    ref_con: *mut c_void,
);

pub type AXValueRef = *const c_void;
