use crate::{
    Bounds, DisplayId, Error, Result,
    bits::{
        CGRect, CGWindowListCopyWindowInfo, SharingType, StoreType, WindowId, WindowListOption,
    },
    display::Display,
};
use core_foundation::{Array, Dictionary};
use std::collections::HashMap;

#[derive(Debug)]
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
#[derive(Debug)]
pub struct Window {
    /// The window's alpha fade level. This number is in the range 0.0 to 1.0,
    /// where 0.0 is fully transparent and 1.0 is fully opaque.
    alpha: UnitFloat,
    /// The coordinates of the rectangle are specified in screen space, where
    /// the origin is in the upper-left corner of the main display.
    bounds: Bounds,
    /// Whether the window is currently onscreen.
    is_on_screen: Option<bool>,
    // The window layer number.
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
    sharing_state: SharingType,
    /// Specifies how the window device buffers drawing commands.
    store_type: StoreType,
}

macro_rules! get_or_error {
    ($dict: expr, $key: expr) => {
        $dict
            .get(&$key)
            .ok_or(Error::CouldNotFindDictionaryKey($key.into()))?
    };
}

#[allow(dead_code)]
impl Window {
    pub fn name(&self) -> Option<&str> {
        if let Some(s) = &self.name {
            Some(s)
        } else {
            None
        }
    }

    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn owner_pid(&self) -> libc::pid_t {
        self.owner_pid
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn is_user_application(&self) -> bool {
        self.layer == 0
    }

    pub fn all_windows() -> Result<Vec<Window>> {
        let array_ref = unsafe {
            CGWindowListCopyWindowInfo(
                WindowListOption::EXCLUDE_DESKTOP_ELEMENTS | WindowListOption::ON_SCREEN_ONLY,
                WindowId::Null,
            )
        };

        Ok(Array::<Dictionary>::try_from(array_ref)
            .map_err(Error::CoreFoundation)?
            .into_iter()
            .filter_map(|dict| Window::try_from(dict).ok())
            .filter(Window::is_user_application)
            .filter(|window| window.is_on_screen.is_some_and(|i| i))
            .collect())
    }

    pub fn get_display_id(&self, ds: &HashMap<DisplayId, Display>) -> Option<DisplayId> {
        ds.iter()
            .map(|(id, display)| (id, Bounds::overlapping_area(&self.bounds, &display.bounds)))
            .filter(|(_, area)| *area > 0.0)
            .max_by(|&(_, area_a), &(_, area_b)| f64::total_cmp(&area_a, &area_b))
            .map(|(id, _)| id)
            .copied()
    }
}

impl TryFrom<Dictionary> for Window {
    type Error = Error;

    fn try_from(dictionary: Dictionary) -> std::result::Result<Self, Self::Error> {
        const ALPHA_DICTIONARY_KEY: &str = "kCGWindowAlpha";
        const BOUNDS_DICTIONARY_KEY: &str = "kCGWindowBounds";
        const IS_ON_SCREEN_DICTIONARY_KEY: &str = "kCGWindowIsOnscreen";
        const LAYER_DICTIONARY_KEY: &str = "kCGWindowLayer";
        const MEMORY_USAGE_BYTES_DICTIONARY_KEY: &str = "kCGWindowMemoryUsage";
        const NAME_DICTIONARY_KEY: &str = "kCGWindowName";
        const NUMBER_DICTIONARY_KEY: &str = "kCGWindowNumber";
        const OWNER_NAME_DICTIONARY_KEY: &str = "kCGWindowOwnerName";
        const OWNER_PID_DICTIONARY_KEY: &str = "kCGWindowOwnerPID";
        const SHARING_STATE_DICTIONARY_KEY: &str = "kCGWindowSharingState";
        const STORE_TYPE_DICTIONARY_KEY: &str = "kCGWindowStoreType";

        Ok(Self {
            alpha: UnitFloat(get_or_error!(dictionary, ALPHA_DICTIONARY_KEY)),
            bounds: dictionary
                .get::<&str, CGRect>(&BOUNDS_DICTIONARY_KEY)
                .ok_or(Error::CouldNotFindDictionaryKey(
                    BOUNDS_DICTIONARY_KEY.into(),
                ))?
                .into(),
            is_on_screen: dictionary.get(&IS_ON_SCREEN_DICTIONARY_KEY),
            layer: get_or_error!(dictionary, LAYER_DICTIONARY_KEY),
            memory_usage_bytes: get_or_error!(dictionary, MEMORY_USAGE_BYTES_DICTIONARY_KEY),
            name: dictionary.get(&NAME_DICTIONARY_KEY),
            number: get_or_error!(dictionary, NUMBER_DICTIONARY_KEY),
            owner_name: dictionary.get(&OWNER_NAME_DICTIONARY_KEY),
            owner_pid: get_or_error!(dictionary, OWNER_PID_DICTIONARY_KEY),
            sharing_state: get_or_error!(dictionary, SHARING_STATE_DICTIONARY_KEY),
            store_type: get_or_error!(dictionary, STORE_TYPE_DICTIONARY_KEY),
        })
    }
}
