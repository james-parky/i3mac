use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::TCFTypeRef;
use core_graphics::display::CFDictionaryRef;
use core_graphics::display::CGWindowListCopyWindowInfo;
use core_graphics::display::{CGDirectDisplayID, CGDisplayBounds};
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
};
use i3mac::Window;

// fn get_windows_on_display(id: CGDirectDisplayID) -> Vec<Window> {
//     let array_ref = unsafe {
//         CGWindowListCopyWindowInfo(
//             kCGWindowListOptionExcludeDesktopElements | kCGWindowListOptionOnScreenOnly,
//             kCGNullWindowID,
//         )
//     };
//
//     if array_ref.is_null() {
//         return vec![];
//     }
//
//     (0..unsafe { CFArrayGetCount(array_ref) })
//         .filter_map(|i| {
//             Window::try_from(unsafe {
//                 CFDictionaryRef::from_void_ptr(CFArrayGetValueAtIndex(array_ref, i))
//             })
//             .ok()
//         })
//         .filter(|w| w.get_display_id().is_some_and(|d| d == id))
//         .filter(Window::is_user_application)
//         .collect()
// }

fn main() {
    match i3mac::coregraphics::active_display_ids() {
        Ok(displays) => {
            println!("displays: {:?}", displays);
            println!("Found {} active display(s):", displays.len());
            for (i, disp) in displays.iter().enumerate() {
                let bounds = i3mac::coregraphics::display_bounds(*disp);
                println!("  [{i}] id={disp:?}  bounds=({bounds:?})");
            }
        }
        Err(err) => {
            eprintln!("CGGetActiveDisplayList failed with error code: {err:?}");
        }
    }
}
