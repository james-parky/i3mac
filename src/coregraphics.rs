use crate::bits::{CGDirectDisplayID, CGDisplayBounds, CGError, CGGetActiveDisplayList, CGRect};
use crate::Result;
use std::ffi::c_uint;

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct DisplayId(u32);

impl From<CGDirectDisplayID> for DisplayId {
    fn from(id: CGDirectDisplayID) -> Self {
        DisplayId(id)
    }
}

pub fn active_display_ids() -> Result<Vec<DisplayId>> {
    const MAX_DISPLAY_COUNT: u32 = 16;

    let mut active_displays = [0; MAX_DISPLAY_COUNT as usize];
    let mut display_count: c_uint = 0;

    let cg_err = unsafe {
        CGGetActiveDisplayList(
            MAX_DISPLAY_COUNT,
            active_displays.as_mut_ptr(),
            &mut display_count,
        )
    };

    if let Some(err) = CGError(cg_err).into() {
        Err(err)
    } else {
        Ok(active_displays
            .into_iter()
            .take(display_count as usize)
            .map(DisplayId)
            .collect())
    }
}

pub fn display_bounds(id: DisplayId) -> Bounds {
    unsafe { CGDisplayBounds(id.0) }.into()
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Bounds {
    pub height: f64,
    pub width: f64,
    pub x: f64,
    pub y: f64,
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

impl From<core_graphics::display::CGRect> for Bounds {
    fn from(value: core_graphics::display::CGRect) -> Self {
        Self {
            height: value.size.height,
            width: value.size.width,
            x: value.origin.x,
            y: value.origin.y,
        }
    }
}
