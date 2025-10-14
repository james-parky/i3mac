use crate::Error;
use core_foundation::{CFArrayRef, CFNumberType, CFTypeRef, cf_type_ref_to_num};
use std::ffi::{c_int, c_uint};
use std::ops::BitOr;

/// A unique identifier for an attached display.
///
/// In Quartz, the term display refers to a graphics hardware system consisting
/// of a framebuffer, a color correction (gamma) table, and possibly an attached
/// monitor. If no monitor is attached, a display is characterized as offline.
///
/// When a monitor is attached, Quartz assigns a unique display identifier (ID).
/// A display ID can persist across processes and typically remains constant
/// until the machine is restarted.
///
/// When assigning a display ID, Quartz considers the following parameters:
/// * Vendor
/// * Model
/// * Serial number
/// * Position in the I/O Kit registry
pub type CGDirectDisplayID = c_uint;

/// A uniform type for result codes returned by functions in `core_graphics`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CGError(pub c_int);

impl CGError {
    /// The requested operation is inappropriate for the arguments passed in, or
    /// the current system state.
    pub const CANNOT_COMPILE: Self = Self(1004);
    /// A general failure occurred.
    pub const FAILURE: Self = Self(1000);
    /// One or more of the arguments passed to a function are invalid. Check for
    /// NULL pointers.
    pub const ILLEGAL_ARGUMENT: Self = Self(1001);
    /// The arguments representing a connection to the window server is invalid.
    pub const INVALID_CONNECTION: Self = Self(1002);
    /// The `CPSProcessorSerNum` or context identifier argument is not valid.
    pub const INVALID_CONTEXT: Self = Self(1003);
    /// The requested operation is not valid for the arguments passed in, or the
    /// current system state.
    pub const INVALID_OPERATION: Self = Self(1010);
    /// The requested operation could not be completed as the indicated
    /// resources were not found.
    pub const NONE_AVAILABLE: Self = Self(1011);
    /// Return value from obsolete function stubs present for binary
    /// compatability, but not typically called.
    pub const NOT_IMPLEMENTED: Self = Self(1006);
    /// An argument passed in has a value that is inappropriate, or which does
    /// not map to a useful operation or value.
    pub const RANGE_CHECK: Self = Self(1007);
    /// The requested operation was completed successfully.
    pub const SUCCESS: Self = Self(0);
    /// A data type or token was encountered that did not match the expected
    /// type or token.
    pub const TYPE_CHECK: Self = Self(1008);
}

/// The basic type for all floating-point values.
type CGFloat = f64;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

/// A structure that contains width and height values.
///
/// A `CGSize` struct is sometimes used to represent a distance vector, rather
/// than physical size. As a vector, its values can be negative.
#[repr(C)]
#[derive(Debug)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
#[derive(Debug)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

/// Specifies whether and how windows are shared between applications.
#[derive(Debug, Default)]
#[repr(u32)]
// Created as part of the Core Graphics ffi; yet are unused.
#[allow(dead_code)]
pub enum SharingType {
    /// The window is not shared.
    #[default]
    None = 0,
    /// The window is shared and its contents can be read by all processes but
    /// modified only by the process that created it.
    ReadOnly = 1,
    /// The window is shared and its contents can be read and modified by any
    /// process.
    ReadWrite = 2,
}

/// Specifies how the window device buffers drawing commands.
#[derive(Debug, Default)]
#[repr(u32)]
// Created as part of the Core Graphics ffi; yet are unused.
#[allow(dead_code)]
pub enum StoreType {
    /// The window uses a buffer, but draws directly to the screen where
    /// possible and to the buffer for obscured portions.
    ///
    /// You should typically not use this mode. It combines the limitations of
    /// `StoreType::NonRetained` with the memory use of `StoreType::Buffered`.
    /// The original NeXTSTEP implementation was an interesting compromise that
    /// worked well with fast memory mapped framebuffers on the CPI bus --
    /// something that hasn't been in general use since around 1994. These tend
    /// to have performance problems.
    ///
    /// In macOS 10.5 and later, requests for retained windows will still result
    /// in the window system creating a buffered window, as that better matches
    /// actual use.
    #[default]
    Retained = 0,
    /// The window draws directly to the screen without using any buffer.
    ///
    /// You should typically not use this mode. It exists primarily for use in
    /// the original Classic Blue Box. It does not support Quartz drawing, alpha
    /// blending, or opacity. Moreover, it does not support hardware
    /// acceleration, and interferes with system-wide display acceleration. If
    /// you use this mode, your application must manage visibility region
    /// clipping itself, and manage repainting on visibility changes.
    NonRetained = 1,
    /// The window draws into a display buffer and then flushes that buffer to
    /// the screen.
    ///
    /// You should typically use this mode. It supports hardware acceleration,
    /// Quartz drawing, and takes advantage of the GPU when possible. It also
    /// supports alpha channel drawing, opacity controls, use the compositor.
    Buffered = 2,
}

impl TryFrom<CFTypeRef> for SharingType {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> Result<Self, Self::Error> {
        // TODO: more specific error?
        cf_type_ref_to_num(value, CFNumberType::INT32).map_err(Error::CoreFoundation)
    }
}

impl TryFrom<CFTypeRef> for StoreType {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> Result<Self, Self::Error> {
        // TODO: more specific error?
        cf_type_ref_to_num(value, CFNumberType::INT32).map_err(Error::CoreFoundation)
    }
}

/// The data type used to specify the options for gathering a list of windows.
#[repr(transparent)]
// Core Graphics describes this as an enum, but Rust does not allow for BitOr
// between enum variants, so we use a new-type wrapper around a c_uint and
// provide constants for what the enum variants would have been.
pub struct WindowListOption(c_uint);

impl WindowListOption {
    /// List all windows, including both onscreen and offscreen windows. When
    /// retrieving a list with this option, the `relative_to_window` argument
    /// should be set to `WindowId::Null`.
    #[allow(dead_code)]
    const ALL: Self = Self(0);
    /// List all windows that are currently onscreen. Windows are returned in
    /// order from front to back. When retrieving a list with this option, the
    /// `relative_to_window` argument should be set to `WindowId::Null`.
    pub const ON_SCREEN_ONLY: Self = Self(1);
    /// List all windows that are currently onscreen and in front of the window
    /// specified in the `relative_to_window` argument. Windows are returned in
    /// order from front to back.
    #[allow(dead_code)]
    const ON_SCREEN_ABOVE_WINDOW: Self = Self(2);
    /// List all windows that are currently onscreen and behind the window
    /// specified in the `relative_to_window` argument. Windows are returned in
    /// order from front to back.
    #[allow(dead_code)]
    const ON_SCREEN_BELOW_WINDOW: Self = Self(4);
    /// Include the specified window (from the `relative_to_window` argument) in
    /// the returned list. You must combine this option with the
    /// `WindowListOption::ON_SCREEN_ABOVE_WINDOW` or
    /// `WindowListOption::ON_SCREEN_BELOW_WINDOW` option to retrieve meaningful
    /// results.
    #[allow(dead_code)]
    const INCLUDING_WINDOW: Self = Self(8);
    /// Exclude any windows from the list that are elements of the desktop,
    /// including the background picture and desktop icons. You may combine this
    /// option with the other options.
    pub const EXCLUDE_DESKTOP_ELEMENTS: Self = Self(16);
}

impl BitOr for WindowListOption {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self((self.0) | (rhs.0))
    }
}

/// The data type used to store window identifiers.
#[repr(u32)]
pub enum WindowId {
    /// A guaranteed invalid window ID.
    Null = 0,
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Provides a list of displays that are active for drawing.
    ///
    /// # Arguments
    ///
    /// * `max_displays` - The size of the `active_displays` array. This value
    /// determines the maximum number of displays the list includes.
    /// * `active_displays` - A pointer to storage you provide for an array of
    /// display IDs. On return, the array contains a list of active displays. If
    /// you pass NULL, on return the display count contains the total number of
    /// active displays.
    /// * `display_count` - A pointer to a display count variable you provide.
    /// On return, the display count contains the actual number of displays the
    /// function added to the `active_displays` array. This value is at most
    /// `max_displays`.
    ///
    /// # Returns
    ///
    /// A result code. To interpret the result code, see `CGError`.
    ///
    /// # Discussion
    ///
    /// The first entry in the list of active displays is the main display. In
    /// case of mirroring, the first entry is the largest drawable display or,
    /// if all are the same size, the display with the greatest pixel depth.
    ///
    /// Note that when using hardware mirroring between displays, only the
    /// primary display is active and appears in the list. When using software
    /// mirroring, all the mirrored displays are active and appear in the list.
    /// For more information about mirroring, see
    /// `CGConfigureDisplayMirrorOfDisplay`.
    // Documentation states this returns a CGError (int32_t) but it is better to
    // return a c_int here and cast it to the above _custom_ CGError to make it
    // easier to convert to Errors or Results.
    pub fn CGGetActiveDisplayList(
        max_displays: c_uint,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut c_uint,
    ) -> c_int;

    /// Returns the bounds of a display in the global display coordinate space.
    ///
    /// # Arguments
    ///
    /// * `display` - The identifier of the display to be accessed.
    ///
    /// # Returns
    ///
    /// The bounds of the display, expressed as a rectangle in the global
    /// display coordinate space (relative to the upper-left corner of the main
    /// display).
    pub fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;

    /// Generates and returns information about the selected windows in the
    /// current user session.
    ///
    /// You can use this function to get detailed information about the
    /// configuration of one or more windows in the current user session. For
    /// example, you can use this function to get the bounds of the window, its
    /// window ID, and information about how it is managed by the window server.
    /// For the list of keys and values that may be present in the dictionary,
    /// see `core_graphics::Display`.
    ///
    /// Generating the dictionaries for system windows is a relatively expensive
    /// operation. As always, you should profile your code and adjust your usage
    /// of this function appropriately for your needs.
    ///
    /// # Arguments
    ///
    /// * `option` - The options describing which window dictionaries to return.
    /// Typical options let you return dictionaries for all windows or for
    /// windows above or below the window specified in the `relative_to_window`
    /// parameter. For more information, see `WindowListOption`.
    /// * `relative_to_window` - The ID of the window to use as a reference
    /// point when determining which other window dictionaries to return. For
    /// options that do not require a reference window, this parameter can be
    /// `WindowID::Null`.
    ///
    /// # Returns
    ///
    /// An array of `CFDictionaryRef` types, each of which contains information
    /// about one of the windows in the current user session. If there are no
    /// windows matching the desired criteria, the function returns an empty
    /// array. If you call this function from outside of a GUI security session
    /// or when no window server is running, this function returns NULL.
    pub fn CGWindowListCopyWindowInfo(
        option: WindowListOption,
        relative_to_window: WindowId,
    ) -> CFArrayRef;
}
