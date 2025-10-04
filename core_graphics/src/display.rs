use crate::bits::{CGDisplayBounds, CGError, CGGetActiveDisplayList};
use crate::window::Window;
use crate::{Bounds, DisplayId, Error};
use std::collections::HashMap;
use std::ffi::c_uint;

#[derive(Debug)]
pub struct Display {
    pub bounds: Bounds,
    pub windows: Vec<Window>,
}

impl Display {
    pub fn all() -> crate::Result<HashMap<DisplayId, Display>> {
        let mut displays: HashMap<DisplayId, Display> = Self::active_display_ids()?
            .into_iter()
            .map(|id| (id, Self::new(id)))
            .collect();

        for window in Window::all_windows()? {
            match window.get_display_id(&displays) {
                Err(err) => return Err(err),
                // TODO: real error
                Ok(Some(id)) => displays
                    .get_mut(&id)
                    .ok_or(Error::NulString)?
                    .windows
                    .push(window),
                Ok(None) => {} // window could not be assigned to a screen; assume it is off-screen
            }
        }

        Ok(displays)
    }

    fn new(id: DisplayId) -> Display {
        Self {
            bounds: unsafe { CGDisplayBounds(id.into()) }.into(),
            windows: Vec::new(),
        }
    }

    fn active_display_ids() -> crate::Result<Vec<DisplayId>> {
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
                .map(Into::into)
                .collect())
        }
    }
}
