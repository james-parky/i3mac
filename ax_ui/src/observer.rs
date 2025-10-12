use crate::bits::{
    AXError, AXObserverAddNotification, AXObserverCallback, AXObserverCreate,
    AXObserverGetRunLoopSource, AXObserverRef, AxUiElementRef,
};
use crate::window::cfstring;
use crate::{Error, Result};
use core_foundation::{CFRunLoopAddSource, CFRunLoopGetCurrent, kCFRunLoopDefaultMode};
use std::ffi::c_void;

#[derive(Debug, Copy, Clone)]
pub struct Observer {
    ax_ref: AXObserverRef,
}

impl Observer {
    pub fn try_new(owner_pid: libc::pid_t, callback: AXObserverCallback) -> Result<Self> {
        let mut observer: AXObserverRef = std::ptr::null_mut();

        match AXError(unsafe { AXObserverCreate(owner_pid, callback, &mut observer) }) {
            AXError::SUCCESS => Ok(Self {
                ax_ref: observer.cast(),
            }),
            err => Err(Error::CouldNotCreateObserver(owner_pid, err)),
        }
    }

    // TODO: event type with associated constants?
    pub fn add_notification(
        &self,
        window_ref: AxUiElementRef,
        event: &str,
        ctx: *mut c_void,
    ) -> Result<()> {
        let event = cfstring(event)?;

        match AXError(unsafe { AXObserverAddNotification(self.ax_ref, window_ref, event, ctx) }) {
            AXError::SUCCESS => {}
            err => return Err(Error::CouldNotAttachNotification(window_ref, err)),
        }

        unsafe {
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                AXObserverGetRunLoopSource(self.ax_ref),
                kCFRunLoopDefaultMode,
            )
        };

        Ok(())
    }
}
