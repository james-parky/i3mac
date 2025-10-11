use crate::bits::{
    AXError, AXObserverAddNotification, AXObserverCallback, AXObserverCreate,
    AXObserverGetRunLoopSource, AXObserverRef, AXObserverRemoveNotification,
    AXUIElementCopyAttributeValue, AXUIElementCreateApplication, AXUIElementSetAttributeValue,
    AXValueCreate, AXValueGetValue, AXValueRef, AXValueType, AxUiElementRef,
};
use crate::{Error, Result};
use core_foundation::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFRunLoopAddSource, CFRunLoopGetCurrent,
    CFStringCreateWithCString, CFStringEncoding, CFStringRef, kCFRunLoopDefaultMode,
};
use core_graphics::{Bounds, CGPoint, CGRect, CGSize};
use std::ffi::c_void;

#[derive(Debug, Copy, Clone)]
pub struct Window {
    owner_pid: libc::pid_t,
    application_ref: AxUiElementRef,
    window_ref: AxUiElementRef,
    observer_ref: Option<AXObserverRef>,
}

impl Window {
    // Not all AXUI window necessarily contain the same unique window ID as core_graphics provides.
    // Unfortunately the most reliable way to match a core_graphics window to an AXUI window using
    // its bounds.
    pub fn new(owner_pid: libc::pid_t, bounds: &Bounds) -> Result<Self> {
        if !pid_exists(owner_pid) {
            return Err(Error::PidDoesNotExist(owner_pid));
        }

        let application_ref = unsafe { AXUIElementCreateApplication(owner_pid) };
        let windows_array_ref = get_window_ref_array(application_ref)?;

        for i in 0..unsafe { CFArrayGetCount(windows_array_ref) } {
            let window_ref =
                unsafe { CFArrayGetValueAtIndex(windows_array_ref, i) } as AxUiElementRef;
            let point = get_window_position(window_ref)?;

            if point.x == bounds.x && point.y == bounds.y {
                return Ok(Self {
                    owner_pid,
                    application_ref,
                    window_ref,
                    observer_ref: None,
                });
            }
        }

        Err(Error::CouldNotFindWindow(owner_pid))
    }
    pub fn attach_lock_callback(&mut self, point: CGPoint, size: CGSize) -> Result<()> {
        if let Some(observer_ref) = self.observer_ref {
            let resized = cfstring("AXResized")?;
            let moved = cfstring("AXMoved")?;
            let _ = unsafe { AXObserverRemoveNotification(observer_ref, self.window_ref, resized) };
            let _ = unsafe { AXObserverRemoveNotification(observer_ref, self.window_ref, moved) };
        }

        let (lock_callback, ctx) = self.create_lock_callback(point, size);

        let mut observer: AXObserverRef = std::ptr::null_mut();
        let res = unsafe { AXObserverCreate(self.owner_pid, lock_callback, &mut observer) };

        if res != 0 {
            // TODO: observer error
            return Err(Error::CouldNotCreateObserver(self.owner_pid));
        }

        let resized = cfstring("AXResized")?;
        let res = unsafe { AXObserverAddNotification(observer, self.window_ref, resized, ctx) };
        // TODO: unique error
        if res != 0 {
            // TODO: observer error
            return Err(Error::CouldNotCreateObserver(self.owner_pid));
        }
        let moved = cfstring("AXMoved")?;
        let res = unsafe { AXObserverAddNotification(observer, self.window_ref, moved, ctx) };
        // TODO: unique error
        if res != 0 {
            // TODO: observer error
            return Err(Error::CouldNotCreateObserver(self.owner_pid));
        }

        unsafe {
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                AXObserverGetRunLoopSource(observer),
                kCFRunLoopDefaultMode,
            )
        };

        self.observer_ref = Some(observer);

        Ok(())
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

    fn create_lock_callback(
        &self,
        point: CGPoint,
        size: CGSize,
    ) -> (AXObserverCallback, *mut c_void) {
        let ctx = Box::new(WindowLockCallbackContext {
            window: self.window_ref,
            point,
            size,
        });

        let ctx_ptr = Box::into_raw(ctx);

        extern "C" fn callback(
            _observer: AXObserverRef,
            _element: AxUiElementRef,
            _notification: CFStringRef,
            context: *mut c_void,
        ) {
            let ctx: &WindowLockCallbackContext =
                unsafe { &*(context as *const WindowLockCallbackContext) };

            let ax_value = unsafe {
                AXValueCreate(
                    AXValueType::CG_POINT,
                    &ctx.point as *const _ as *const c_void,
                )
            };
            // TODO: handle
            let res = unsafe {
                AXUIElementSetAttributeValue(
                    ctx.window,
                    cfstring("AXPosition").unwrap(),
                    ax_value.cast(),
                )
            };
            let ax_value = unsafe {
                AXValueCreate(AXValueType::CG_SIZE, &ctx.size as *const _ as *const c_void)
            };
            // TODO: handle
            let res = unsafe {
                AXUIElementSetAttributeValue(
                    ctx.window,
                    cfstring("AXSize").unwrap(),
                    ax_value.cast(),
                )
            };
        }

        (callback, ctx_ptr as *mut c_void)
    }
}

#[repr(C)]
struct WindowLockCallbackContext {
    window: AxUiElementRef,
    point: CGPoint,
    size: CGSize,
}

// TODO: newtype for CfStringRef and impl a TryFrom<&str>
fn cfstring(s: &str) -> Result<CFStringRef> {
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

fn get_window_position(window: AxUiElementRef) -> Result<CGPoint> {
    let position_attr = cfstring("AXPosition")?;
    let mut number_ref: *const c_void = std::ptr::null_mut();

    if let Some(err) =
        AXError(unsafe { AXUIElementCopyAttributeValue(window, position_attr, &mut number_ref) })
            .into()
    {
        return Err(err);
    }

    let mut point = CGPoint { x: 0.0, y: 0.0 };
    if !unsafe {
        AXValueGetValue(
            number_ref as AXValueRef,
            AXValueType::CG_POINT,
            &mut point as *mut _ as *mut c_void,
        )
    } {
        // TODO: error for not fetching value
        Err(Error::Unknown)
    } else {
        Ok(point)
    }
}
