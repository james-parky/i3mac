mod bits;
mod display;
mod error;
mod keyboard;
mod window;

pub use bits::CGPoint;
pub use bits::CGRect;
pub use bits::CGSize;
pub use bits::CGWarpMouseCursorPosition;
pub use bits::WindowId;
pub use keyboard::{Direction, KeyCommand, KeyboardHandler};
use std::hash::{Hash, Hasher};
pub use window::Window;

use core_foundation::{CFDictionaryRef, CFTypeRef, Dictionary};
pub use display::*;

use crate::bits::CGDirectDisplayID;
pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
#[allow(dead_code)]
pub struct DisplayId(u32);

impl From<CGDirectDisplayID> for DisplayId {
    fn from(id: CGDirectDisplayID) -> Self {
        DisplayId(id)
    }
}

impl From<DisplayId> for CGDirectDisplayID {
    fn from(id: DisplayId) -> Self {
        id.0
    }
}

impl From<usize> for DisplayId {
    fn from(id: usize) -> Self {
        DisplayId(id as u32)
    }
}

impl std::fmt::Display for DisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct Bounds {
    pub height: f64,
    pub width: f64,
    pub x: f64,
    pub y: f64,
}

impl PartialEq for Bounds {
    fn eq(&self, other: &Self) -> bool {
        self.x.to_bits() == other.x.to_bits()
            && self.y.to_bits() == other.y.to_bits()
            && self.width.to_bits() == other.width.to_bits()
            && self.height.to_bits() == other.height.to_bits()
    }
}

impl Eq for Bounds {}

impl Hash for Bounds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

impl Bounds {
    pub fn with_pad(&self, pad: f64) -> Self {
        Self {
            height: self.height - (2.0 * pad),
            width: self.width - (2.0 * pad),
            x: self.x + pad,
            y: self.y + pad,
        }
    }

    pub fn overlapping_area(a: &Bounds, b: &Bounds) -> f64 {
        let ix = a.x.max(b.x);
        let iy = a.y.max(b.y);
        let iw = (a.x + a.width).min(b.x + b.width) - ix;
        let ih = (a.y + a.height).min(b.y + b.height) - iy;

        if iw > 0.0 && ih > 0.0 { iw * ih } else { 0.0 }
    }

    pub fn point(&self) -> CGPoint {
        CGPoint {
            x: self.x,
            y: self.y,
        }
    }

    pub fn size(&self) -> CGSize {
        CGSize {
            width: self.width,
            height: self.height,
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

impl TryFrom<CFTypeRef> for CGRect {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> Result<Self> {
        if value.0.is_null() {
            return Err(Error::NulString);
        }
        let dict: CFDictionaryRef = value.0 as CFDictionaryRef;
        // TODO: more specific?
        let d = Dictionary::try_from(dict).map_err(Error::CoreFoundation)?;

        // TODO: proper errors
        let x: f64 = d.get(&"X").ok_or(Error::NulString)?;
        let y: f64 = d.get(&"Y").ok_or(Error::NulString)?;
        let width: f64 = d.get(&"Width").ok_or(Error::NulString)?;
        let height: f64 = d.get(&"Height").ok_or(Error::NulString)?;

        Ok(CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width, height },
        })
    }
}
