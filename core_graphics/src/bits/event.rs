use std::ffi::{c_ulonglong, c_void};
use std::ops::{BitAnd, Shl};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn CGEventTapCreate(
        tap: EventTapLocation,
        place: EventTapPlacement,
        options: EventTapOptions,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    pub fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        tap: CFMachPortRef,
        order: isize,
    ) -> *mut c_void;

    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;

    pub fn CGEventGetFlags(event: CGEventRef) -> EventFlags;
}

#[repr(u32)]
pub enum EventTapLocation {
    Hid = 0,
    Session = 1,
}

#[repr(u32)]
pub enum EventTapPlacement {
    HeadInsert = 0,
    TailAppend = 1,
}

#[repr(u32)]
pub enum EventTapOptions {
    Default = 0,
    ListenOnly = 1,
}

#[derive(PartialEq)]
#[repr(u32)]
pub enum EventType {
    KeyDown = 10,
    KeyUp = 11,
    FlagsChanged = 12,
}

pub type CGEventMask = u64;
pub type CGEventRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;
pub type CFMachPortRef = *mut c_void;
pub type CGKeyCode = u16;

#[repr(transparent)]
pub struct EventFlags(c_ulonglong);

impl EventFlags {
    pub fn has_alpha_shift(&self) -> bool {
        self.0 & 0x0001_0000 != 0
    }

    pub fn has_shift(&self) -> bool {
        self.0 & 0x0002_0000 != 0
    }

    pub fn has_control(&self) -> bool {
        self.0 & 0x0004_0000 != 0
    }

    pub fn has_alt(&self) -> bool {
        self.0 & 0x0008_0000 != 0
    }

    pub fn has_command(&self) -> bool {
        self.0 & 0x0010_0000 != 0
    }

    pub fn has_secondary_fn(&self) -> bool {
        self.0 & 0x0080_0000 != 0
    }
}

pub type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    event_type: EventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

// Event field constants
pub const kCGKeyboardEventKeycode: u32 = 9;

impl Shl<EventType> for u64 {
    type Output = u64;

    fn shl(self, rhs: EventType) -> Self::Output {
        self << (rhs as u32)
    }
}
