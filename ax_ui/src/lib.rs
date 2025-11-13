mod bits;
mod error;
mod observer;
mod window;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use observer::Observer;
pub use observer::*;
pub use window::Window;

pub use bits::AXIsProcessTrusted;

use core_foundation::{CFStringCreateWithCString, CFStringEncoding, CFStringRef};

/// Create a `CFString` for a `&str` and return a pointer to it.
///
/// # Arguments
///
/// * `s` - The string.
///
/// # Returns
///
/// A `CFStringRef` pointer to the `CFString` created from `s`.
///
/// # Safety
///
/// * When you are finished using the `CFStringRef` returned by this function,
///   you must call `CFRelease()` on it to avoid memory leaking.
pub(crate) unsafe fn try_create_cf_string(s: &str) -> Result<CFStringRef> {
    unsafe {
        let cstr = std::ffi::CString::new(s).map_err(Error::CannotMakeCString)?;

        Ok(CFStringCreateWithCString(
            std::ptr::null(),
            cstr.as_ptr(),
            CFStringEncoding::Utf8,
        ))
    }
}
