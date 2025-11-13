use crate::bits::{AXUIElementCreateSystemWide, AXUIElementGetPid, AXUIElementPerformAction};
use crate::{
    Error, Result,
    bits::{
        _AXUIElementGetWindow, AXError, AXUIElementCopyAttributeValue,
        AXUIElementCreateApplication, AXUIElementSetAttributeValue, AXValueCreate, AXValueGetValue,
        AXValueRef, AXValueType, AxUiElementRef,
    },
};
use core_foundation::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFRelease, CFRetain,
    CFStringCreateWithCString, CFStringEncoding, CFStringRef, CFTypeRef, kCFBooleanTrue,
};
use core_graphics::{CGPoint, CGSize, WindowId};
use libc::raise;
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

    pub fn new(owner_pid: libc::pid_t, cg_window_number: WindowId) -> Result<Self> {
        if !pid_exists(owner_pid) {
            return Err(Error::PidDoesNotExist(owner_pid));
        }

        let application_ref = unsafe { AXUIElementCreateApplication(owner_pid) };
        let windows_array_ref = get_window_ref_array(application_ref)?;

        for i in 0..unsafe { CFArrayGetCount(windows_array_ref) } {
            let window_ref =
                unsafe { CFArrayGetValueAtIndex(windows_array_ref, i) } as AxUiElementRef;

            let ax_window_number =
                get_window_id(window_ref).ok_or(Error::CouldNotGetWindowNumber(window_ref))?;

            if ax_window_number == cg_window_number {
                unsafe { CFRetain(CFTypeRef(window_ref)) };
                unsafe {
                    CFRelease(CFTypeRef(windows_array_ref));
                }

                return Ok(Self {
                    owner_pid,
                    application_ref,
                    window_ref,
                });
            }
        }

        unsafe {
            CFRelease(CFTypeRef(windows_array_ref));
        }

        Err(Error::CouldNotFindWindow(owner_pid))
    }

    pub fn min_size(&self) -> Result<CGSize> {
        let attr_name = cfstring(Self::MIN_SIZE_ATTR)?;

        let mut value = std::ptr::null();
        let result =
            unsafe { AXUIElementCopyAttributeValue(self.window_ref, attr_name, &mut value) };

        unsafe { CFRelease(CFTypeRef(attr_name)) };

        if let Some(err) = AXError(result).into() {
            return Err(err);
        }

        let mut size = CGSize {
            width: 0.0,
            height: 0.0,
        };

        let success = unsafe {
            AXValueGetValue(
                value as AXValueRef,
                AXValueType::CG_SIZE,
                &mut size as *mut _ as *mut c_void,
            )
        };

        unsafe { CFRelease(CFTypeRef(value)) };

        if !success {
            Err(Error::CouldNotGetWindowSize(self.application_ref))
        } else {
            Ok(size)
        }
    }

    pub fn window_ref(&self) -> AxUiElementRef {
        self.window_ref
    }

    pub fn move_to(&self, x: f64, y: f64) -> Result<()> {
        let pos_attr = cfstring(Self::POSITION_ATTR)?;
        let point = CGPoint { x, y };
        let ax_value =
            unsafe { AXValueCreate(AXValueType::CG_POINT, &point as *const _ as *const c_void) };

        let result = unsafe {
            AXUIElementSetAttributeValue(self.window_ref, pos_attr, ax_value as *const c_void)
        };

        unsafe { CFRelease(CFTypeRef(ax_value)) };
        unsafe { CFRelease(CFTypeRef(pos_attr)) };

        match AXError(result).into() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn resize(&self, width: f64, height: f64) -> Result<()> {
        let size_attr = cfstring(Self::SIZE_ATTR)?;
        let point = CGSize { width, height };
        let ax_value =
            unsafe { AXValueCreate(AXValueType::CG_SIZE, &point as *const _ as *const c_void) };

        let result = unsafe {
            AXUIElementSetAttributeValue(self.window_ref, size_attr, ax_value as *const c_void)
        };

        unsafe { CFRelease(CFTypeRef(ax_value)) };
        unsafe { CFRelease(CFTypeRef(size_attr)) };

        match AXError(result).into() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn focus(&self) -> Result<()> {
        let raise_attr = cfstring("AXRaise")?;
        let result = unsafe { AXUIElementPerformAction(self.window_ref, raise_attr) };

        unsafe { CFRelease(CFTypeRef(raise_attr)) };

        if result != 0 {
            return Err(Error::CouldNotFocusWindow(self.window_ref));
        }

        let attr_name = cfstring("AXMain")?;
        let result =
            unsafe { AXUIElementSetAttributeValue(self.window_ref, attr_name, kCFBooleanTrue) };
        unsafe { CFRelease(CFTypeRef(attr_name)) };
        if result != 0 {
            return Err(Error::CouldNotFocusWindow(self.window_ref));
        }

        // Also try to focus it
        let focused_attr = cfstring("AXFocused")?;
        let result =
            unsafe { AXUIElementSetAttributeValue(self.window_ref, focused_attr, kCFBooleanTrue) };
        unsafe { CFRelease(CFTypeRef(focused_attr)) };
        if result != 0 {
            return Err(Error::CouldNotFocusWindow(self.window_ref));
        }

        let frontmost_attr = cfstring("AXFrontmost")?;
        let result = unsafe {
            AXUIElementSetAttributeValue(self.application_ref, frontmost_attr, kCFBooleanTrue)
        };
        unsafe { CFRelease(CFTypeRef(frontmost_attr)) };
        if result != 0 {
            return Err(Error::CouldNotFocusWindow(self.window_ref));
        }

        Ok(())
    }

    pub fn get_focused() -> Result<WindowId> {
        let system_wide = unsafe { AXUIElementCreateSystemWide() };
        let focused_attr = cfstring("AXFocusedUIElement")?;

        let mut focused_element: *const c_void = std::ptr::null();
        let result = unsafe {
            AXUIElementCopyAttributeValue(system_wide, focused_attr, &mut focused_element)
        };
        unsafe { CFRelease(CFTypeRef(focused_attr)) };
        unsafe { CFRelease(CFTypeRef(system_wide)) };

        // TODO: handle properly
        if result != 0 {
            println!("{:?}", AXError(result));
            return Err(Error::CouldNotGetFocusedWindow);
        }

        let window_result = get_window_id(focused_element);

        unsafe { CFRelease(CFTypeRef(focused_element)) };

        match window_result {
            Some(window) => Ok(window),
            None => Err(Error::CouldNotGetWindowNumber(focused_element)),
        }
    }
}

// TODO: newtype for CfStringRef and impl a TryFrom<&str>
pub(crate) fn cfstring(s: &str) -> Result<CFStringRef> {
    let cstr = std::ffi::CString::new(s).map_err(Error::CannotMakeCString)?;
    Ok(unsafe {
        CFStringCreateWithCString(std::ptr::null(), cstr.as_ptr(), CFStringEncoding::Utf8)
    })
}

fn pid_exists(pid: libc::pid_t) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

fn get_window_ref_array(application_ref: AxUiElementRef) -> Result<CFArrayRef> {
    let windows_attr = cfstring(Window::WINDOWS_ATTR)?;
    let mut value: *const c_void = std::ptr::null();

    let result =
        unsafe { AXUIElementCopyAttributeValue(application_ref, windows_attr, &mut value) };

    unsafe { CFRelease(CFTypeRef(windows_attr)) };

    match AXError(result).into() {
        Some(e) => Err(e),
        None => Ok(value as CFArrayRef),
    }
}

fn get_window_id(window: AxUiElementRef) -> Option<WindowId> {
    unsafe {
        let mut window_id = WindowId::NULL;
        let result = _AXUIElementGetWindow(CFTypeRef(window), &mut window_id);
        if result == 0 { Some(window_id) } else { None }
    }
}
