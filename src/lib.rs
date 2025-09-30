use core_graphics::window::{
    kCGWindowBackingStoreBuffered as K_CG_WINDOW_BACKING_STORE_BUFFERED,
    kCGWindowBackingStoreNonretained as K_CG_WINDOW_BACKING_STORE_NON_RETAINED,
    kCGWindowBackingStoreRetained as K_CG_WINDOW_BACKING_STORE_RETAINED,
};

use core_graphics::window::{
    kCGWindowSharingNone as K_CG_WINDOW_SHARING_NONE,
    kCGWindowSharingReadOnly as K_CG_WINDOW_SHARING_READ_ONLY,
    kCGWindowSharingReadWrite as K_CG_WINDOW_SHARING_READ_WRITE,
};

pub struct UnitFloat(f32);

impl UnitFloat {
    pub fn new(value: f32) -> Option<UnitFloat> {
        if (0.0..=1.0).contains(&value) {
            Some(UnitFloat(value))
        } else {
            None
        }
    }

    pub fn inner(&self) -> f32 {
        self.0
    }
}

#[allow(dead_code)]
pub struct Window {
    /// The window's alpha fade level. This number is in the range 0.0 to 1.0,
    /// where 0.0 is fully transparent and 1.0 is fully opaque.
    alpha: UnitFloat,
    /// The coordinates of the rectangle are specified in screen space, where
    /// the origin is in the upper-left corner of the main display.
    bounds: Bounds,
    /// Whether the window is currently onscreen.
    is_on_screen: Option<bool>,
    /// The window layer number.
    layer: i32,
    /// An estimate of the amount of memory (measured in bytes) used by the
    /// window and its supporting data structures.
    memory_usage_bytes: u64,
    /// The name of the window, as configured in Quartz. (Note that few
    /// applications set the Quartz window name.)
    name: Option<String>,
    /// The window ID; unique within the current user session.
    number: u64,
    /// The name of the application that owns the window.
    owner_name: Option<String>,
    /// The process ID of the applications that owns the window.
    owner_pid: libc::pid_t,
    /// Specifies whether and how windows are shared between applications.
    sharing_state: SharingState,
    /// Specifies how the window device buffers drawing commands.
    store_type: StoreType,
}

#[allow(dead_code)]
impl Window {
    const ALPHA_DICTIONARY_KEY: &'static str = "kCGWindowAlpha";
    const BOUNDS_DICTIONARY_KEY: &'static str = "kCGWindowBounds";
    const IS_ON_SCREEN_DICTIONARY_KEY: &'static str = "kCGWindowIsOnscreen";
    const LAYER_DICTIONARY_KEY: &'static str = "kCGWindowLayer";
    const MEMORY_USAGE_BYTES_DICTIONARY_KEY: &'static str = "kCGWindowMemoryUsage";
    const NAME_DICTIONARY_KEY: &'static str = "kCGWindowName";
    const NUMBER_DICTIONARY_KEY: &'static str = "kCGWindowNumber";
    const OWNER_NAME_DICTIONARY_KEY: &'static str = "kCGWindowOwnerName";
    const OWNER_PID_DICTIONARY_KEY: &'static str = "kCGWindowOwnerPID";
    const SHARING_STATE_DICTIONARY_KEY: &'static str = "kCGWindowSharingState";
    const STORE_TYPE_DICTIONARY_KEY: &'static str = "kCGWindowStoreType";
}

pub enum StoreType {
    Retained,
    NonRetained,
    Buffered,
}

impl TryFrom<CGWindowBackingType> for StoreType {
    type Error = &'static str;
    fn try_from(value: CGWindowBackingType) -> Result<Self, Self::Error> {
        match value {
            K_CG_WINDOW_BACKING_STORE_RETAINED => Ok(StoreType::Retained),
            K_CG_WINDOW_BACKING_STORE_NON_RETAINED => Ok(StoreType::NonRetained),
            K_CG_WINDOW_BACKING_STORE_BUFFERED => Ok(StoreType::Buffered),
            _ => Err("unknown window backing store: {value:?}"),
        }
    }
}

impl From<StoreType> for CGWindowBackingType {
    fn from(value: StoreType) -> Self {
        match value {
            StoreType::Retained => K_CG_WINDOW_BACKING_STORE_RETAINED,
            StoreType::NonRetained => K_CG_WINDOW_BACKING_STORE_NON_RETAINED,
            StoreType::Buffered => K_CG_WINDOW_BACKING_STORE_BUFFERED,
        }
    }
}

pub enum SharingState {
    None,
    ReadOnly,
    ReadWrite,
}

impl TryFrom<CGWindowSharingType> for SharingState {
    type Error = &'static str;
    fn try_from(value: CGWindowSharingType) -> Result<Self, Self::Error> {
        // There are three available constants regarding window sharing
        // state. I don't know why ReadOnly and ReadWrite are the same, but
        // I ignore it in case it changes.
        #[allow(clippy::match_overlapping_arm)]
        match value {
            K_CG_WINDOW_SHARING_NONE => Ok(SharingState::None),
            K_CG_WINDOW_SHARING_READ_ONLY => Ok(SharingState::ReadOnly),
            #[allow(unreachable_patterns)]
            K_CG_WINDOW_SHARING_READ_WRITE => Ok(SharingState::ReadWrite),
            _ => Err("unknown window sharing state: {value:?}"),
        }
    }
}

impl From<SharingState> for CGWindowSharingType {
    fn from(value: SharingState) -> Self {
        match value {
            SharingState::None => K_CG_WINDOW_SHARING_NONE,
            SharingState::ReadOnly => K_CG_WINDOW_SHARING_READ_ONLY,
            SharingState::ReadWrite => K_CG_WINDOW_SHARING_READ_WRITE,
        }
    }
}

#[allow(dead_code)]
pub struct Bounds {
    height: f64,
    width: f64,
    x: f64,
    y: f64,
}
