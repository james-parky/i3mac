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

#[allow(dead_code)]
impl Window {
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

        if array_ref.is_null() {
            // TODO: more specific error
            return Err(Error::NullCFArray);
        }

        let window_dict_array = Array::try_create(array_ref).map_err(Error::CoreFoundation)?;

        let mut vec = Vec::with_capacity(window_dict_array.len());

        for i in 0..window_dict_array.len() {
            let dict: Dictionary = window_dict_array.get(i).map_err(Error::CoreFoundation)?;
            let window = Window::try_from(dict)?;
            if window.is_user_application() && window.is_user_application() {
                vec.push(window);
            }
        }

        Ok(vec)
    }

    pub fn get_display_id(
        &self,
        ds: &HashMap<DisplayId, Display>,
    ) -> crate::Result<Option<DisplayId>> {
        let mut best: Option<DisplayId> = None;
        let mut max_area = 0.0;

        for (id, disp) in ds {
            let area = Bounds::overlapping_area(&self.bounds, &disp.bounds);
            if area > max_area {
                max_area = area;
                best = Some(*id);
            }
        }

        Ok(best)
    }
}

impl TryFrom<Dictionary> for Window {
    // TODO: proper error type
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

        // TODO: errors
        Ok(Self {
            alpha: UnitFloat(
                dictionary
                    .get(&ALPHA_DICTIONARY_KEY)
                    .ok_or(Error::NullCFArray)?,
            ),
            bounds: dictionary
                .get::<&str, CGRect>(&BOUNDS_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?
                .into(),
            is_on_screen: dictionary.get(&IS_ON_SCREEN_DICTIONARY_KEY),
            layer: dictionary
                .get(&LAYER_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
            memory_usage_bytes: dictionary
                .get(&MEMORY_USAGE_BYTES_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
            name: dictionary.get(&NAME_DICTIONARY_KEY),
            number: dictionary
                .get(&NUMBER_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
            owner_name: dictionary.get(&OWNER_NAME_DICTIONARY_KEY),
            owner_pid: dictionary
                .get(&OWNER_PID_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
            sharing_state: dictionary
                .get(&SHARING_STATE_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
            store_type: dictionary
                .get(&STORE_TYPE_DICTIONARY_KEY)
                .ok_or(Error::NullCFArray)?,
        })
    }
}
