use core_foundation::base::{CFTypeRef, TCFType, ToVoid};
use core_foundation::dictionary::{CFDictionary, CFDictionaryGetValueIfPresent};
use core_foundation::number::{
    kCFNumberFloatType, kCFNumberIntType, kCFNumberLongLongType, CFBooleanGetValue,
    CFNumberGetValue, CFNumberType,
};
use core_foundation::string::CFString;
use core_graphics::base::CGError;
use core_graphics::display::{
    CFDictionaryRef, CGDirectDisplayID, CGDisplayBounds, CGGetActiveDisplayList, CGRect,
};
use core_graphics::window::{
    kCGWindowBackingStoreBuffered as K_CG_WINDOW_BACKING_STORE_BUFFERED,
    kCGWindowBackingStoreNonretained as K_CG_WINDOW_BACKING_STORE_NON_RETAINED,
    kCGWindowBackingStoreRetained as K_CG_WINDOW_BACKING_STORE_RETAINED, CGWindowBackingType,
    CGWindowSharingType,
};

use core_graphics::window::{
    kCGWindowSharingNone as K_CG_WINDOW_SHARING_NONE,
    kCGWindowSharingReadOnly as K_CG_WINDOW_SHARING_READ_ONLY,
    kCGWindowSharingReadWrite as K_CG_WINDOW_SHARING_READ_WRITE,
};

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

use core_graphics::base::{
    kCGErrorFailure as K_CG_ERROR_FAILURE, kCGErrorSuccess as K_CG_ERROR_SUCCESS,
};

pub fn get_active_displays() -> Result<Vec<CGDirectDisplayID>, CGError> {
    const MAX_DISPLAYS: u32 = 16;

    let mut buffer: Vec<CGDirectDisplayID> = vec![0u32; MAX_DISPLAYS as usize];
    let mut actual_count: u32 = 0;

    match unsafe {
        CGGetActiveDisplayList(MAX_DISPLAYS, buffer.as_mut_ptr(), &raw mut actual_count)
    } {
        K_CG_ERROR_SUCCESS if actual_count <= MAX_DISPLAYS => {
            Ok(buffer[0..actual_count as usize].to_vec())
        }
        K_CG_ERROR_SUCCESS if actual_count > MAX_DISPLAYS => Err(K_CG_ERROR_FAILURE),
        x => Err(x),
    }
}

#[allow(dead_code)]
impl Window {
    pub fn is_user_application(&self) -> bool {
        self.layer == 0
    }

    pub fn get_display_id(&self) -> Option<CGDirectDisplayID> {
        let buffer = get_active_displays().ok()?;

        let mut best: Option<u32> = None;
        let mut max_area = 0.0;

        for id in &buffer {
            let rect = unsafe { CGDisplayBounds(*id) };
            let area = Bounds::overlapping_area(&self.bounds, &rect.into());
            if area > max_area {
                max_area = area;
                best = Some(*id);
            }
        }

        best
    }
}

fn get_number_from_dict<T: Default>(
    dict: CFDictionaryRef,
    key: &str,
    conversion_const: u32,
) -> Result<T, &'static str> {
    get_from_dict(dict, key, |value_ref| unsafe {
        let mut val = T::default();
        CFNumberGetValue(value_ref.cast(), conversion_const, (&raw mut val).cast());
        val
    })
}

fn get_boolean_from_dict(dict: CFDictionaryRef, key: &str) -> Result<bool, &'static str> {
    get_from_dict(dict, key, |value_ref| unsafe {
        CFBooleanGetValue(value_ref.cast())
    })
}

fn get_string_from_dict(dict: CFDictionaryRef, key: &str) -> Result<String, &'static str> {
    get_from_dict(dict, key, |value_ref| unsafe {
        CFString::wrap_under_get_rule(value_ref.cast()).to_string()
    })
}

fn get_bounds_from_dict(dict: CFDictionaryRef, key: &str) -> Result<Bounds, &'static str> {
    get_from_dict(dict, key, |value_ref| {
        let bounds_dict: CFDictionary =
            unsafe { CFDictionary::wrap_under_get_rule(value_ref.cast()) };

        CGRect::from_dict_representation(&bounds_dict)
            .expect("could not get bounds")
            .into()
    })
}

fn get_from_dict<T, F>(dict: CFDictionaryRef, key: &str, conv: F) -> Result<T, &'static str>
where
    F: FnOnce(CFTypeRef) -> T,
{
    let mut val_ref: CFTypeRef = std::ptr::null_mut();
    if unsafe {
        CFDictionaryGetValueIfPresent(dict, CFString::new(key).to_void(), &raw mut val_ref)
    } != 1
    {
        return Err("failed to get");
    }

    Ok(conv(val_ref))
}

impl TryFrom<CFDictionaryRef> for Window {
    type Error = &'static str;
    fn try_from(dict: CFDictionaryRef) -> Result<Self, Self::Error> {
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

        // Allow this to make it consistent with constants imported from
        // core_foundation.
        #[allow(non_upper_case_globals)]
        const kCGWindowIDCFNumberType: CFNumberType = kCFNumberLongLongType;

        let alpha = get_number_from_dict(dict, ALPHA_DICTIONARY_KEY, kCFNumberFloatType)?;

        Ok(Self {
            alpha: UnitFloat::new(alpha).expect("invalid alpha"),
            bounds: get_bounds_from_dict(dict, BOUNDS_DICTIONARY_KEY)?,
            is_on_screen: get_boolean_from_dict(dict, IS_ON_SCREEN_DICTIONARY_KEY).ok(),
            layer: get_number_from_dict(dict, LAYER_DICTIONARY_KEY, kCFNumberIntType)?,
            memory_usage_bytes: get_number_from_dict(
                dict,
                MEMORY_USAGE_BYTES_DICTIONARY_KEY,
                kCFNumberLongLongType,
            )?,
            name: get_string_from_dict(dict, NAME_DICTIONARY_KEY).ok(),
            number: get_number_from_dict(dict, NUMBER_DICTIONARY_KEY, kCGWindowIDCFNumberType)?,
            owner_name: get_string_from_dict(dict, OWNER_NAME_DICTIONARY_KEY).ok(),
            owner_pid: get_number_from_dict::<libc::pid_t>(
                dict,
                OWNER_PID_DICTIONARY_KEY,
                kCFNumberIntType,
            )?,
            sharing_state: get_number_from_dict::<CGWindowSharingType>(
                dict,
                SHARING_STATE_DICTIONARY_KEY,
                kCFNumberIntType,
            )?
            .try_into()?,
            store_type: get_number_from_dict::<CGWindowBackingType>(
                dict,
                STORE_TYPE_DICTIONARY_KEY,
                kCFNumberIntType,
            )?
            .try_into()?,
        })
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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
#[derive(Debug)]
pub struct Bounds {
    height: f64,
    width: f64,
    x: f64,
    y: f64,
}

impl Bounds {
    pub fn overlapping_area(a: &Bounds, b: &Bounds) -> f64 {
        let ix = a.x.max(b.x);
        let iy = a.y.max(b.y);
        let iw = (a.x + a.width).min(b.x + b.width) - ix;
        let ih = (a.y + a.height).min(b.y + b.height) - iy;

        if iw > 0.0 && ih > 0.0 {
            iw * ih
        } else {
            0.0
        }
    }
}

impl From<CGRect> for Bounds {
    fn from(value: CGRect) -> Self {
        Self {
            height: value.size.height,
            width: value.size.width,
            x: value.origin.x,
            y: value.origin.y,
        }
    }
}
