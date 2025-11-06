use crate::{
    Error, Result,
    bits::{
        _AXUIElementGetWindow, AXError, AXUIElementCopyAttributeValue,
        AXUIElementCreateApplication, AXUIElementSetAttributeValue, AXValueCreate, AXValueGetValue,
        AXValueRef, AXValueType, AxUiElementRef,
    },
};
use core_foundation::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFStringCreateWithCString,
    CFStringEncoding, CFStringRef, CFTypeRef,
};
use core_graphics::{CGPoint, CGSize, WindowId};
use std::ffi::c_void;

#[derive(Debug, Copy, Clone, Hash)]
pub struct Window {
    owner_pid: libc::pid_t,
    application_ref: AxUiElementRef,
    window_ref: AxUiElementRef,
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
                return Ok(Self {
                    owner_pid,
                    application_ref,
                    window_ref,
                });
            }
        }

        Err(Error::CouldNotFindWindow(owner_pid))
    }

    pub fn min_size(&self) -> Result<CGSize> {
        let attr_name = cfstring(Self::MIN_SIZE_ATTR)?;

        let mut value = std::ptr::null();
        let result =
            unsafe { AXUIElementCopyAttributeValue(self.window_ref, attr_name, &mut value) };

        if let Some(err) = AXError(result).into() {
            return Err(err);
        }

        let mut size = CGSize {
            width: 0.0,
            height: 0.0,
        };

        if !unsafe {
            AXValueGetValue(
                value as AXValueRef,
                AXValueType::CG_SIZE,
                &mut size as *mut _ as *mut c_void,
            )
        } {
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

        match AXError(unsafe {
            AXUIElementSetAttributeValue(self.window_ref, pos_attr, ax_value as *const c_void)
        })
        .into()
        {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn resize(&self, width: f64, height: f64) -> Result<()> {
        let size_attr = cfstring(Self::SIZE_ATTR)?;
        let point = CGSize { width, height };
        let ax_value =
            unsafe { AXValueCreate(AXValueType::CG_SIZE, &point as *const _ as *const c_void) };

        match AXError(unsafe {
            AXUIElementSetAttributeValue(self.window_ref, size_attr, ax_value as *const c_void)
        })
        .into()
        {
            Some(err) => Err(err),
            None => Ok(()),
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

    match AXError(unsafe {
        AXUIElementCopyAttributeValue(application_ref, windows_attr, &mut value)
    })
    .into()
    {
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
