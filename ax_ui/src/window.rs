use crate::{
    Error, Result,
    bits::{
        _AXUIElementGetWindow, AXError, AXUIElementCopyAttributeValue,
        AXUIElementCreateApplication, AXUIElementCreateSystemWide, AXUIElementPerformAction,
        AXUIElementSetAttributeValue, AXValueCreate, AXValueRef, AXValueType, AxUiElementRef,
    },
    try_create_cf_string,
};
use core_foundation::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFRelease, CFRetain, CFTypeRef,
    kCFBooleanFalse, kCFBooleanTrue,
};
use core_graphics::{CGPoint, CGSize, WindowId};
use std::ffi::c_void;

#[derive(Debug, Hash)]
pub struct Window {
    owner_pid: libc::pid_t,
    application_ref: AxUiElementRef,
    window_ref: AxUiElementRef,
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            CFRelease(CFTypeRef(self.application_ref));
            CFRelease(CFTypeRef(self.window_ref));
        }
    }
}

impl Clone for Window {
    fn clone(&self) -> Self {
        unsafe {
            CFRetain(CFTypeRef(self.application_ref));
            CFRetain(CFTypeRef(self.window_ref));
        }

        Self {
            owner_pid: self.owner_pid,
            application_ref: self.application_ref,
            window_ref: self.window_ref,
        }
    }
}

impl Window {
    const MIN_SIZE_ATTR: &'static str = "AXMinSize";
    const POSITION_ATTR: &'static str = "AXPosition";
    const SIZE_ATTR: &'static str = "AXSize";
    const WINDOWS_ATTR: &'static str = "AXWindows";
    pub const RESIZED_ATTR: &'static str = "AXResized";
    pub const MOVED_ATTR: &'static str = "AXMoved";

    const FOCUSED_UI_ELEMENT_ATTR: &'static str = "AXFocusedUIElement";
    const RAISE_ATTR: &'static str = "AXRaise";
    const MAIN_ATTR: &'static str = "AXMain";
    const FOCUS_ATTR: &'static str = "AXFocused";
    const FRONTMOST_ATTR: &'static str = "AXFrontmost";

    pub fn new(owner_pid: libc::pid_t, cg_window_number: WindowId) -> Result<Self> {
        if !pid_exists(owner_pid) {
            return Err(Error::PidDoesNotExist(owner_pid));
        }

        // Safety:
        //  - `application_ref` is released on error path, or stored in returned
        //    `Self` on success.
        //  - `windows_array_ref` is released before return.
        //  - `window_ref` is retained before the array is released.
        unsafe {
            let application_ref = AXUIElementCreateApplication(owner_pid);
            let windows_array_ref = Window::try_get_window_ref_array(application_ref)?;

            for i in 0..CFArrayGetCount(windows_array_ref) {
                let window_ref = CFArrayGetValueAtIndex(windows_array_ref, i);

                let window_id =
                    Window::get_window_id(window_ref).ok_or(Error::CouldNotGetWindowId)?;

                if window_id == cg_window_number {
                    CFRetain(CFTypeRef(window_ref));
                    CFRelease(CFTypeRef(windows_array_ref));

                    return Ok(Self {
                        owner_pid,
                        application_ref,
                        window_ref,
                    });
                }
            }

            CFRelease(CFTypeRef(windows_array_ref));
            CFRelease(CFTypeRef(application_ref));
        }

        Err(Error::CouldNotFindWindow(owner_pid))
    }

    /// Get the minimum size the `Window` can be on-screen as a `CGSize`.
    pub fn min_size(&self) -> Result<CGSize> {
        try_get_attr(self.window_ref, Self::MIN_SIZE_ATTR)
    }

    /// Get the reference of the `Windows` internal window object.
    pub fn window_ref(&self) -> AxUiElementRef {
        self.window_ref
    }

    // TODO: take CGPoint
    /// Try to move the `Window` to (x, y).
    pub fn try_move_to(&self, x: f64, y: f64) -> Result<()> {
        try_set_attr(self.window_ref, Self::POSITION_ATTR, CGPoint { x, y })
    }

    // TODO: take CGSize
    /// Try to resize the `Window` to the given width and height.
    pub fn try_resize(&self, width: f64, height: f64) -> Result<()> {
        try_set_attr(self.window_ref, Self::SIZE_ATTR, CGSize { width, height })
    }

    /// Try to focus this window.
    pub fn try_focus(&self) -> Result<()> {
        try_perform_action(self.window_ref, Window::RAISE_ATTR)?;
        try_set_attr(self.window_ref, Window::MAIN_ATTR, true)?;
        try_set_attr(self.window_ref, Window::FOCUS_ATTR, true)?;
        try_set_attr(self.application_ref, Window::FRONTMOST_ATTR, true)?;

        Ok(())
    }

    /// Get the ID of the currently focused window.
    pub fn try_get_focused() -> Result<WindowId> {
        // Safety:
        //  - `system_wide` is released before return.
        //  - `attr` is released before return.
        //  - `focused_element` is released before return, and is valid for the
        //    call to `get_window_id()`.
        unsafe {
            let system_wide = AXUIElementCreateSystemWide();
            let attr = try_create_cf_string(Window::FOCUSED_UI_ELEMENT_ATTR)?;

            let mut focused_element: *const c_void = std::ptr::null();
            let result = AXUIElementCopyAttributeValue(system_wide, attr, &mut focused_element);

            CFRelease(CFTypeRef(attr));
            CFRelease(CFTypeRef(system_wide));

            if let Some(err) = AXError(result).into() {
                return Err(err);
            }

            let focused_window_id = Window::get_window_id(focused_element);

            CFRelease(CFTypeRef(focused_element));

            focused_window_id.ok_or(Error::CouldNotGetWindowId)
        }
    }

    /// Get the ID of the window from an element pointer referencing it.
    ///
    /// # Safety
    ///
    /// * `window_ref` must be a valid `AxUiElementRef` value, a pointer to an
    ///   Application Services window.
    // TODO: handle all return values from call and return Result<WindowID>. Might
    //       be tricky given its a private API.
    unsafe fn get_window_id(window_ref: AxUiElementRef) -> Option<WindowId> {
        // Safety:
        //  - `window_ref` validity is guaranteed by the caller contract.
        unsafe {
            let mut window_id = WindowId::NULL;

            if _AXUIElementGetWindow(CFTypeRef(window_ref), &mut window_id) == 0 {
                Some(window_id)
            } else {
                None
            }
        }
    }

    /// Returns a reference to the array containing information on the windows owned
    /// by the supplied application.
    ///
    /// # Safety
    ///
    /// * `application_ref` must be a valid `AxUiElementRef` value, a pointer to an
    ///   Application Services application.
    unsafe fn try_get_window_ref_array(application_ref: AxUiElementRef) -> Result<CFArrayRef> {
        // Safety:
        //  - `attr` is released before return.
        //  - `application_ref` validity is guaranteed as per function contract, as
        //    is not modified before use.
        //  - `value` is only used as an output parameter, and is guaranteed to be
        //    castable to `CFArrayRef`.
        unsafe {
            let attr = try_create_cf_string(Window::WINDOWS_ATTR)?;
            let mut value = std::ptr::null();

            let result = AXUIElementCopyAttributeValue(application_ref, attr, &mut value);

            CFRelease(CFTypeRef(attr));

            match AXError(result).into() {
                Some(e) => Err(e),
                None => Ok(value as CFArrayRef),
            }
        }
    }
}

enum Value {
    Borrowed(AXValueRef),
    Owned(AXValueRef),
}

impl Value {
    fn as_ptr(&self) -> *const c_void {
        match self {
            Self::Borrowed(v) | Self::Owned(v) => v.0,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        let ax_ref = if value {
            unsafe { kCFBooleanTrue }
        } else {
            unsafe { kCFBooleanFalse }
        };

        Self::Borrowed(AXValueRef(ax_ref))
    }
}

impl From<CGSize> for Value {
    fn from(value: CGSize) -> Self {
        unsafe {
            let ptr = AXValueCreate(AXValueType::CG_SIZE, &value as *const _ as *const c_void);
            Self::Owned(ptr)
        }
    }
}

impl From<CGPoint> for Value {
    fn from(value: CGPoint) -> Self {
        unsafe {
            let ptr = AXValueCreate(AXValueType::CG_POINT, &value as *const _ as *const c_void);
            Self::Owned(ptr)
        }
    }
}

/// Check if a given `pid` exists or not.
fn pid_exists(pid: libc::pid_t) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Try to set the attribute `attr` on the `Window`.
fn try_set_attr<T>(element_ref: AxUiElementRef, attr: &str, value: T) -> Result<()>
where
    T: Into<Value>,
{
    // Safety:
    //  - `cf_attr` is released before return.
    //  - `self.window_ref` is checked for nullity before use.
    unsafe {
        if element_ref.is_null() {
            return Err(Error::WindowRefIsNull);
        }

        let ax_value: Value = value.into();
        let cf_attr = try_create_cf_string(attr)?;

        let result = AXUIElementSetAttributeValue(element_ref, cf_attr, ax_value.as_ptr());

        CFRelease(CFTypeRef(cf_attr));

        if let Value::Owned(ax_ref) = ax_value {
            CFRelease(CFTypeRef(ax_ref.0));
        }

        match AXError(result).into() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

/// Try to get the attribute `attr` from the `Window`.
fn try_get_attr<T>(element_ref: AxUiElementRef, attr: &str) -> Result<T>
where
    T: TryFrom<AXValueRef, Error = Error>,
{
    // Safety:
    //  - `cf_attr` is released before return.
    //  - `value` is released before return.
    //  - `self.window_ref` validity is guaranteed by `Window`'s invariants.
    unsafe {
        if element_ref.is_null() {
            return Err(Error::WindowRefIsNull);
        }

        let cf_attr = try_create_cf_string(attr)?;
        let mut value = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(element_ref, cf_attr, &mut value);

        CFRelease(CFTypeRef(cf_attr));

        if let Some(err) = AXError(result).into() {
            return Err(err);
        }

        let ret = T::try_from(AXValueRef(value));

        CFRelease(CFTypeRef(value));

        ret
    }
}

/// Try to perform the action with string `attr` on the `Window`.
fn try_perform_action(element_ref: AxUiElementRef, attr: &str) -> Result<()> {
    // Safety:
    //  - `attr` is released before return.
    //  - `self.window_ref` is checked for nullity before use.
    unsafe {
        if element_ref.is_null() {
            return Err(Error::WindowRefIsNull);
        }

        let cf_attr = try_create_cf_string(attr)?;
        let result = AXUIElementPerformAction(element_ref, cf_attr);
        CFRelease(CFTypeRef(cf_attr));

        match AXError(result).into() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}
