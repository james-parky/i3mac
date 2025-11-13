use core_foundation::CFMachPortRef;
use std::{
    ffi::{c_ulonglong, c_void},
    ops::Shl,
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Creates an event tap.
    ///
    /// # Arguments
    ///
    /// * `tap` - The location of the new event tap. Pass one of the variants in
    ///   `EventTapLocation`. Only processes running as the root user may locate
    ///   an event tap at the point where HID events enter the window server;
    ///   for other users, this function return `NULL`.
    /// * `place` - The placement of the new event tap in the list of active
    ///   event taps. Pass one of the variants in `EventTapPlacement`.
    /// * `options` - A constant that specifies whether the new event tap is a
    ///   passive listener or an active filter.
    /// * `events_of_interest` - A bit mask that specifies the set of events to
    ///   be observed. For a list of possible events, see `CGEventMask`. If the
    ///   event tap is not permitted to monitor one or more of the events
    ///   specified in the `events_of_interest` argument, then the appropriate
    ///   bits in the mask are cleared. If that action results in an empty mask,
    ///   this function returns `NULL`.
    /// * `callback` - An event tap callback function that you provide. Your
    ///   callback function is invoked from the run loop to which the event tap
    ///   is added as a source. The thread safety of the callback is defined by
    ///   the run loop's environment. To learn more about event tap callbacks,
    ///   see `CGEventTapCallback`.
    /// * `user_info` - A pointer to user-defined data. This pointer is passed
    ///   into the callback function specified in the `callback` parameter.
    ///
    /// # Returns
    ///
    /// A `core_graphics` mach port that represents the new event tap, or `NULL`
    /// if the event tap could not be created. When you are finished using the
    /// event tap, you should release the mach port using the function
    /// `CFRelease`. Releasing the mach port also releases the tap.
    ///
    /// # Discussion
    ///
    /// Event taps receive key up and key down events if one of the following
    /// conditions is true:
    ///
    /// * The current process is running as the root user.
    /// * Access for assistive devices in enabled. In OS X v10.4, you can enable
    ///   this feature using System Preferences, Universal Access panel,
    ///   Keyboard view.
    ///
    /// After creating an event tap, you can add it to a run loop as follows:
    ///
    /// 1. Pass the event tap to the `CFMachPortCreateRunLoopSource` function to
    ///    create a run loop event source.
    /// 2. Call the `CFRunLoopAddSource` function to add the source to the
    ///    appropriate run loop.
    pub fn CGEventTapCreate(
        tap: EventTapLocation,
        place: EventTapPlacement,
        options: EventTapOptions,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    /// Enables or disables an event tap.
    ///
    /// # Arguments
    ///
    /// * `tap` - The event tap to enable or disable.
    /// * `enable` - Pass `true` to enable the event tap. To disable it, pass
    ///   `false`.
    ///
    /// # Discussion
    ///
    /// Event taps are normally enabled when created. If an event tap becomes
    /// unresponsive, or it a user requests that event taps be disabled, then a
    /// `kCGEventTapDisabled` event is passed to the event tap callback
    /// function. Event taps may be re-enabled by calling this function.
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    /// Returns the integer value of a field in a Quartz event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to access.
    /// * `field` - A field in the specified event. Pass one of the constants
    ///   listed in `CGEventField`.
    ///
    /// # Returns
    ///
    /// A 64-bit integer representation of the current value of the specified
    /// field.
    // TODO: create EventType enum rather than u32 and add constants in
    //       https://developer.apple.com/documentation/coregraphics/cgeventfield?language=objc
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;

    /// Returns the event flags of a Quartz event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to access.
    ///
    /// # Returns
    ///
    /// The current flags of the specified event. For more information, see
    /// `EventFlags`.
    pub fn CGEventGetFlags(event: CGEventRef) -> EventFlags;
}

// TODO: document
#[repr(u32)]
pub enum EventTapLocation {
    #[allow(dead_code)]
    Hid = 0,
    Session = 1,
}

// TODO: document
#[repr(u32)]
pub enum EventTapPlacement {
    HeadInsert = 0,
    #[allow(dead_code)]
    TailAppend = 1,
}

// TODO: document
#[repr(u32)]
pub enum EventTapOptions {
    Default = 0,
    #[allow(dead_code)]
    ListenOnly = 1,
}

// TODO: document
#[derive(PartialEq)]
#[repr(u32)]
pub enum EventType {
    // TODO: fill out from https://developer.apple.com/documentation/coregraphics/cgeventtype?language=objc?
    KeyDown = 10,
    KeyUp = 11,
    FlagsChanged = 12,
}

// TODO: document
pub type CGEventMask = u64;
// TODO: document
pub type CGEventRef = *mut c_void;
// TODO: document
pub type CGEventTapProxy = *mut c_void;

// TODO: document
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

// TODO: document
pub type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    event_type: EventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

// TODO: does this come from a constant that can be extern "C"-ed?
pub const KEYBOARD_EVENT_KEYCODE: u32 = 9;

impl Shl<EventType> for u64 {
    type Output = u64;

    fn shl(self, rhs: EventType) -> Self::Output {
        self << (rhs as u32)
    }
}
