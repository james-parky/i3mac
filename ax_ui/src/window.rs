use crate::Error::CoreFoundation;
use crate::bits::{
    AXError, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXUIElementSetAttributeValue, AXValueCreate, AXValueType, AxUiElementRef,
};
use crate::{Error, Result};
use core_foundation::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFIndex, CFStringCreateWithCString,
    CFStringEncoding, CFStringGetCString, CFStringGetLength, CFStringGetMaximumSizeForEncoding,
    CFStringRef,
};
use core_graphics::{CGPoint, CGSize};
use std::ffi::{CStr, c_void};

#[derive(Debug, Copy, Clone)]
pub struct Window {
    owner_pid: libc::pid_t,
    application_ref: AxUiElementRef,
    window_ref: AxUiElementRef,
}

impl Window {
    // Not all AXUI window necessarily contain the same unique window ID as core_graphics provides.
    // Unfortunately the most reliable way to match a core_graphics window to an AXUI window using
    // its name.
    pub fn new(owner_pid: libc::pid_t, search_title: String) -> Result<Self> {
        if !pid_exists(owner_pid) {
            return Err(Error::PidDoesNotExist(owner_pid));
        }

        let application_ref = unsafe { AXUIElementCreateApplication(owner_pid) };
        let windows_array_ref = get_window_ref_array(application_ref)?;

        for i in 0..unsafe { CFArrayGetCount(windows_array_ref) } {
            let window_ref =
                unsafe { CFArrayGetValueAtIndex(windows_array_ref, i) } as AxUiElementRef;

            // TODO: so flakey...
            let title = get_window_title(window_ref)?;
            if title.contains(&search_title) {
                return Ok(Self {
                    owner_pid,
                    application_ref,
                    window_ref,
                });
            }
        }

        Err(Error::CouldNotFindWindow(owner_pid))
    }

    pub fn window_ref(&self) -> AxUiElementRef {
        self.window_ref
    }

    pub fn move_to(&self, x: f64, y: f64) -> Result<()> {
        let pos_attr = cfstring("AXPosition")?;
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
        let size_attr = cfstring("AXSize")?;
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
    let windows_attr = cfstring("AXWindows")?;
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

fn get_window_title(window: AxUiElementRef) -> Result<String> {
    let attr = cfstring("AXTitle")?;
    let mut value: *const c_void = std::ptr::null();
    if let Some(err) =
        AXError(unsafe { AXUIElementCopyAttributeValue(window, attr, &mut value) }).into()
    {
        return Err(err);
    }

    let len: CFIndex = unsafe { CFStringGetLength(value as CFStringRef) };
    let max_size = unsafe { CFStringGetMaximumSizeForEncoding(len, CFStringEncoding::Utf8) };

    let mut buffer = vec![0u8; max_size as usize];
    let success = unsafe {
        CFStringGetCString(
            value as CFStringRef,
            buffer.as_mut_ptr().cast(),
            max_size,
            CFStringEncoding::Utf8,
        )
    };

    if success {
        let cstr = unsafe { CStr::from_ptr(buffer.as_ptr().cast()) };
        cstr.to_str()
            .map(String::from)
            .map_err(|e| CoreFoundation(core_foundation::Error::InvalidCString(e)))
    } else {
        // TODO: specific error type
        Err(CoreFoundation(core_foundation::Error::NulString))
    }
}
