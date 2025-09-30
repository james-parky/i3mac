use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::{CFTypeRef, TCFType, TCFTypeRef, ToVoid};
use core_foundation::number::{kCFNumberIntType, CFNumberGetValue};
use core_foundation::string::CFString;
use core_graphics::base::{
    kCGErrorFailure as K_CG_ERROR_FAILURE, kCGErrorSuccess as K_CG_ERROR_SUCCESS,
};
use core_graphics::display::CGError;
use core_graphics::display::CGWindowListCopyWindowInfo;
use core_graphics::display::{CFDictionary, CGDirectDisplayID, CGDisplayBounds};
use core_graphics::display::{
    CFDictionaryGetValueIfPresent, CFDictionaryRef, CGGetActiveDisplayList,
};
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
};

fn get_active_displays() -> Result<Vec<CGDirectDisplayID>, CGError> {
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

fn get_windows_on_display(id: &CGDirectDisplayID) -> Vec<CFDictionaryRef> {
    let array_ref = unsafe {
        CGWindowListCopyWindowInfo(
            kCGWindowListOptionExcludeDesktopElements | kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
        )
    };

    if array_ref.is_null() {
        return vec![];
    }

    (0..unsafe { CFArrayGetCount(array_ref) })
        .map(|i| unsafe { CFDictionaryRef::from_void_ptr(CFArrayGetValueAtIndex(array_ref, i)) })
        .filter(|&w| unsafe { window_on_display(*id, w) })
        .filter(|&w| window_is_user_application(w))
        .collect()
}

fn window_is_user_application(window: CFDictionaryRef) -> bool {
    const WINDOW_LAYER_KEY: &str = "kCGWindowLayer";

    let mut layer_ref: CFTypeRef = std::ptr::null_mut();

    let layer: i32 = if unsafe {
        CFDictionaryGetValueIfPresent(
            window,
            CFString::new(WINDOW_LAYER_KEY).to_void(),
            &raw mut layer_ref,
        )
    } == 1
    {
        let mut l = 0;
        unsafe { CFNumberGetValue(layer_ref.cast(), kCFNumberIntType, (&raw mut l).cast()) };
        l
    } else {
        1
    };

    layer == 0
}

unsafe fn window_on_display(id: CGDirectDisplayID, window: CFDictionaryRef) -> bool {
    get_display_from_window(window).is_some_and(|d| d == id)
}

unsafe fn get_display_from_window(window: CFDictionaryRef) -> Option<CGDirectDisplayID> {
    let key_bounds = CFString::new("kCGWindowBounds");
    let mut bounds: CFTypeRef = std::ptr::null_mut();
    if CFDictionaryGetValueIfPresent(
        window,
        key_bounds.as_concrete_TypeRef().to_void(),
        &raw mut bounds,
    ) == 0
    {
        return None;
    }

    let bound_dict: CFDictionary = CFDictionary::wrap_under_get_rule(bounds as CFDictionaryRef);

    let get_double = |key: &str| -> f64 {
        let mut val: CFTypeRef = std::ptr::null_mut();
        if CFDictionaryGetValueIfPresent(
            bound_dict.as_concrete_TypeRef(),
            CFString::new(key).to_void(),
            &raw mut val,
        ) == 1
        {
            let mut num: i32 = 0;
            CFNumberGetValue(val.cast(), kCFNumberIntType, (&raw mut num).cast());
            f64::from(num)
        } else {
            0.0
        }
    };

    let x = get_double("X");
    let y = get_double("Y");
    let h = get_double("Height");
    let w = get_double("Width");

    let buffer = get_active_displays().ok()?;

    let mut best: Option<u32> = None;
    let mut max_area = 0.0;

    for id in &buffer {
        let rect = CGDisplayBounds(*id);

        let ix = x.max(rect.origin.x);
        let iy = y.max(rect.origin.y);
        let iw = (x + w).min(rect.origin.x + rect.size.width) - ix;
        let ih = (y + h).min(rect.origin.y + rect.size.height) - iy;
        let area = if iw > 0.0 && ih > 0.0 { iw * ih } else { 0.0 };
        if area > max_area {
            max_area = area;
            best = Some(*id);
        }
    }

    best
}

fn main() {
    match get_active_displays() {
        Ok(displays) => {
            println!("Found {} active display(s):", displays.len());
            for (i, disp) in displays.iter().enumerate() {
                let bounds = unsafe { CGDisplayBounds(*disp) };
                println!(
                    "  [{}] id={}  origin=({}, {})  size=({}, {})",
                    i,
                    disp,
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height
                );

                let windows = get_windows_on_display(disp);
                println!("windows on display: {disp}: {windows:?}");
            }
        }
        Err(err) => {
            eprintln!("CGGetActiveDisplayList failed with error code: {err}");
        }
    }
}
