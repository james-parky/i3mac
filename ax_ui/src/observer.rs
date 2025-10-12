use crate::bits::{
    AXError, AXObserverAddNotification, AXObserverCallback, AXObserverCreate,
    AXObserverGetRunLoopSource, AXObserverRef, AxUiElementRef,
};
use crate::window::cfstring;
use crate::{Error, Result};
use core_foundation::{
    CFRunLoopAddSource, CFRunLoopGetCurrent, CFStringRef, kCFRunLoopDefaultMode,
};
use std::ffi::c_void;

#[derive(Debug, Copy, Clone)]
pub struct Observer {
    ax_ref: AXObserverRef,
}

impl Observer {
    pub fn try_new(owner_pid: libc::pid_t, callback: &Callback) -> Result<Self> {
        let mut observer: AXObserverRef = std::ptr::null_mut();

        match AXError(unsafe { AXObserverCreate(owner_pid, callback.func, &mut observer) }) {
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

        Ok(())
    }

    pub fn run(&self) {
        unsafe {
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                AXObserverGetRunLoopSource(self.ax_ref),
                kCFRunLoopDefaultMode,
            )
        };
    }
}

pub struct Context<D, F>
where
    F: FnMut(&D),
{
    data: D,
    body: F,
}

#[derive(Debug, Copy, Clone)]
pub struct Callback {
    pub func: AXObserverCallback,
    pub ctx: *mut c_void,
}

impl Callback {
    pub fn new<D, F>(data: D, body: F) -> Self
    where
        F: FnMut(&D),
    {
        let ctx = Box::new(Context { data, body });
        let ctx_ptr = Box::into_raw(ctx);

        extern "C" fn callback<D, F>(
            _observer: AXObserverRef,
            _element: AxUiElementRef,
            _notification: CFStringRef,
            context: *mut c_void,
        ) where
            F: FnMut(&D),
        {
            let ctx: &mut Context<D, F> = unsafe { &mut *(context as *mut Context<D, F>) };

            (ctx.body)(&ctx.data);
        }

        Self {
            func: callback::<D, F>,
            ctx: ctx_ptr as *mut c_void,
        }
    }
}
