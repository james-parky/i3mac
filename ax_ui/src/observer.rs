use crate::{
    Error, Result,
    bits::{
        AXError, AXObserverAddNotification, AXObserverCallback, AXObserverCreate,
        AXObserverGetRunLoopSource, AXObserverRef, AXObserverRemoveNotification, AxUiElementRef,
    },
    try_create_cf_string,
};
use core_foundation::{
    CFRelease, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRemoveSource, CFStringRef,
    CFTypeRef, kCFRunLoopDefaultMode,
};
use std::ffi::c_void;

#[derive(Debug, Clone, Hash)]
pub struct Observer {
    ax_ref: AXObserverRef,
}

impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            let source = AXObserverGetRunLoopSource(self.ax_ref);
            CFRunLoopRemoveSource(CFRunLoopGetCurrent(), source, kCFRunLoopDefaultMode);
            CFRelease(CFTypeRef(self.ax_ref));
        }
    }
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

    // TODO: safety statement
    #[allow(clippy::missing_safety_doc)]
    // TODO: event type with associated constants?
    pub unsafe fn add_notification(
        &self,
        window_ref: AxUiElementRef,
        event: &str,
        ctx: *mut c_void,
    ) -> Result<()> {
        let event = unsafe { try_create_cf_string(event) }?;
        match AXError(unsafe { AXObserverAddNotification(self.ax_ref, window_ref, event, ctx) }) {
            AXError::SUCCESS => Ok(()),
            err => Err(Error::CouldNotAttachNotification(window_ref, err)),
        }
    }

    // TODO: safety statement
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn remove_notification(
        &self,
        window_ref: AxUiElementRef,
        event: &str,
    ) -> Result<()> {
        let event = unsafe { try_create_cf_string(event) }?;

        match AXError(unsafe { AXObserverRemoveNotification(self.ax_ref, window_ref, event) }) {
            AXError::SUCCESS => Ok(()),
            err => Err(Error::CouldNotRemoveNotification(window_ref, err)),
        }
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

#[derive(Debug, Hash, Clone)]
pub struct Callback {
    pub func: AXObserverCallback,
    pub ctx: *mut c_void,
    drop: unsafe fn(*mut c_void),
}

impl Drop for Callback {
    fn drop(&mut self) {
        unsafe {
            // let _ = Box::from_raw(self.ctx);
            (self.drop)(self.ctx);
        }
    }
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

        unsafe fn drop_context<D, F>(ptr: *mut c_void)
        where
            F: FnMut(&D),
        {
            unsafe {
                let _ = Box::from_raw(ptr as *mut Context<D, F>);
            }
        }

        Self {
            func: callback::<D, F>,
            ctx: ctx_ptr as *mut c_void,
            drop: drop_context::<D, F>,
        }
    }
}
