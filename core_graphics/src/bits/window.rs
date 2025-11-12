use core_foundation::CFArrayRef;
use std::{
    ffi::c_uint,
    hash::{Hash, Hasher},
    ops::BitOr,
};

#[derive(Debug, Copy, Clone)]
/// The data type used to store window identifiers.
#[repr(transparent)]
pub struct WindowId(c_uint);
impl WindowId {
    /// A guaranteed invalid window ID.
    pub const NULL: Self = Self(0);
}

impl Hash for WindowId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq<WindowId> for WindowId {
    fn eq(&self, rhs: &WindowId) -> bool {
        self.0 == rhs.0
    }
}

impl Eq for WindowId {}

impl From<u64> for WindowId {
    fn from(value: u64) -> Self {
        Self(value as c_uint)
    }
}

impl From<u32> for WindowId {
    fn from(value: u32) -> Self {
        Self(value as c_uint)
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

pub const ALPHA_DICTIONARY_KEY: &str = "kCGWindowAlpha";
pub const BOUNDS_DICTIONARY_KEY: &str = "kCGWindowBounds";
pub const IS_ON_SCREEN_DICTIONARY_KEY: &str = "kCGWindowIsOnscreen";
pub const LAYER_DICTIONARY_KEY: &str = "kCGWindowLayer";
pub const MEMORY_USAGE_BYTES_DICTIONARY_KEY: &str = "kCGWindowMemoryUsage";
pub const NAME_DICTIONARY_KEY: &str = "kCGWindowName";
pub const NUMBER_DICTIONARY_KEY: &str = "kCGWindowNumber";
pub const OWNER_NAME_DICTIONARY_KEY: &str = "kCGWindowOwnerName";
pub const OWNER_PID_DICTIONARY_KEY: &str = "kCGWindowOwnerPID";
pub const SHARING_STATE_DICTIONARY_KEY: &str = "kCGWindowSharingState";
pub const STORE_TYPE_DICTIONARY_KEY: &str = "kCGWindowStoreType";

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
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
    ///   Typical options let you return dictionaries for all windows or for
    ///   windows above or below the window specified in the
    ///   `relative_to_window` parameter. For more information, see
    ///   `WindowListOption`.
    /// * `relative_to_window` - The ID of the window to use as a reference
    ///   point when determining which other window dictionaries to return. For
    ///   options that do not require a reference window, this parameter can be
    ///   `WindowID::Null`.
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
