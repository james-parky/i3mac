use crate::poll::error::Error;
use crate::poll::{ChannelSender, Result};
use core_foundation::{
    CFMachPortCreateRunLoopSource, CFMachPortRef, CFRelease, CFRunLoopAddSource, CFRunLoopMode,
    CFRunLoopRef, CFTypeRef,
};
use core_graphics::{
    CGEventGetFlags, CGEventGetIntegerValueField, CGEventRef, CGEventTapCreate, CGEventTapEnable,
    CGEventTapProxy, Direction, EventTapLocation, EventTapOptions, EventTapPlacement, EventType,
    HotKey, KEYBOARD_EVENT_KEYCODE, KeyCommand, Keycode, Modifier,
};
use std::os::raw::c_void;

pub struct KeyboardHandler {
    event_tap: CFMachPortRef,
    sender_ptr: *mut ChannelSender<KeyCommand>,
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        unsafe {
            CGEventTapEnable(self.event_tap, false);
            CFRelease(CFTypeRef(self.event_tap));
            let _ = Box::from_raw(self.sender_ptr);
        }
    }
}

impl KeyboardHandler {
    pub fn new(command_sender: ChannelSender<KeyCommand>) -> Result<Self> {
        let sender_ptr = Box::into_raw(Box::new(command_sender));

        let event_mask = 1 << EventType::KeyDown;

        let event_tap = unsafe {
            CGEventTapCreate(
                EventTapLocation::Session,
                EventTapPlacement::HeadInsert,
                EventTapOptions::Default,
                event_mask,
                event_callback,
                sender_ptr as *mut c_void,
            )
        };

        if event_tap.is_null() {
            unsafe {
                let _ = Box::from_raw(sender_ptr);
            }
            return Err(Error::FailedToCreateKeyboardEventTap);
        }

        unsafe { CGEventTapEnable(event_tap, true) };

        Ok(Self {
            event_tap,
            sender_ptr,
        })
    }

    /// # Safety
    ///
    /// * `run_loop` must be a non-null pointer to a Core Graphics run loop.
    pub unsafe fn add_to_run_loop(
        &self,
        run_loop: CFRunLoopRef,
        mode: CFRunLoopMode,
    ) -> Result<()> {
        unsafe {
            let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), self.event_tap, 0);
            if source.is_null() {
                return Err(Error::FailedToCreateRunLoopSource);
            }

            CFRunLoopAddSource(run_loop, source, mode);
            CFRelease(CFTypeRef(source));
            Ok(())
        }
    }
}

pub extern "C" fn event_callback(
    _proxy: CGEventTapProxy,
    event_type: EventType,
    event: CGEventRef,
    user_info: *mut std::ffi::c_void,
) -> CGEventRef {
    if event_type != EventType::KeyDown {
        return event;
    }

    let flags = unsafe { CGEventGetFlags(event) };
    let keycode_val = unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) };
    let keycode: Keycode = match keycode_val.try_into() {
        Ok(x) => x,
        Err(_) => return event,
    };

    let sender = unsafe { &*(user_info as *mut ChannelSender<KeyCommand>) };
    let modifiers: Vec<Modifier> = flags.into();

    let command = match HotKey::new(&modifiers, keycode) {
        HotKey::RESIZE_LEFT => Some(KeyCommand::ResizeWindow(Direction::Left)),
        HotKey::RESIZE_RIGHT => Some(KeyCommand::ResizeWindow(Direction::Right)),
        HotKey::RESIZE_UP => Some(KeyCommand::ResizeWindow(Direction::Up)),
        HotKey::RESIZE_DOWN => Some(KeyCommand::ResizeWindow(Direction::Down)),

        HotKey::OPEN_TERMINAL => Some(KeyCommand::NewTerminal),

        HotKey::FOCUS_LEFT => Some(KeyCommand::Focus(Direction::Left)),
        HotKey::FOCUS_RIGHT => Some(KeyCommand::Focus(Direction::Right)),
        HotKey::FOCUS_UP => Some(KeyCommand::Focus(Direction::Up)),
        HotKey::FOCUS_DOWN => Some(KeyCommand::Focus(Direction::Down)),

        HotKey::MOVE_WINDOW_LEFT => Some(KeyCommand::MoveWindow(Direction::Left)),
        HotKey::MOVE_WINDOW_RIGHT => Some(KeyCommand::MoveWindow(Direction::Right)),
        HotKey::MOVE_WINDOW_UP => Some(KeyCommand::MoveWindow(Direction::Up)),
        HotKey::MOVE_WINDOW_DOWN => Some(KeyCommand::MoveWindow(Direction::Down)),

        HotKey::FOCUS_DISPLAY_0 => Some(KeyCommand::FocusDisplay(0)),
        HotKey::FOCUS_DISPLAY_1 => Some(KeyCommand::FocusDisplay(1)),
        HotKey::FOCUS_DISPLAY_2 => Some(KeyCommand::FocusDisplay(2)),
        HotKey::FOCUS_DISPLAY_3 => Some(KeyCommand::FocusDisplay(3)),
        HotKey::FOCUS_DISPLAY_4 => Some(KeyCommand::FocusDisplay(4)),
        HotKey::FOCUS_DISPLAY_5 => Some(KeyCommand::FocusDisplay(5)),
        HotKey::FOCUS_DISPLAY_6 => Some(KeyCommand::FocusDisplay(6)),
        HotKey::FOCUS_DISPLAY_7 => Some(KeyCommand::FocusDisplay(7)),
        HotKey::FOCUS_DISPLAY_8 => Some(KeyCommand::FocusDisplay(8)),
        HotKey::FOCUS_DISPLAY_9 => Some(KeyCommand::FocusDisplay(9)),

        HotKey::MOVE_TO_DISPLAY_0 => Some(KeyCommand::MoveWindowToDisplay(0)),
        HotKey::MOVE_TO_DISPLAY_1 => Some(KeyCommand::MoveWindowToDisplay(1)),
        HotKey::MOVE_TO_DISPLAY_2 => Some(KeyCommand::MoveWindowToDisplay(2)),
        HotKey::MOVE_TO_DISPLAY_3 => Some(KeyCommand::MoveWindowToDisplay(3)),
        HotKey::MOVE_TO_DISPLAY_4 => Some(KeyCommand::MoveWindowToDisplay(4)),
        HotKey::MOVE_TO_DISPLAY_5 => Some(KeyCommand::MoveWindowToDisplay(5)),
        HotKey::MOVE_TO_DISPLAY_6 => Some(KeyCommand::MoveWindowToDisplay(6)),
        HotKey::MOVE_TO_DISPLAY_7 => Some(KeyCommand::MoveWindowToDisplay(7)),
        HotKey::MOVE_TO_DISPLAY_8 => Some(KeyCommand::MoveWindowToDisplay(8)),
        HotKey::MOVE_TO_DISPLAY_9 => Some(KeyCommand::MoveWindowToDisplay(9)),

        HotKey::VERTICAL_SPLIT => Some(KeyCommand::ToggleVerticalSplit),
        HotKey::HORIZONTAL_SPLIT => Some(KeyCommand::ToggleHorizontalSplit),

        HotKey::TOGGLE_FLOATING => Some(KeyCommand::ToggleFloating),

        _ => None,
    };

    if let Some(c) = command {
        sender.send(c);
        return std::ptr::null_mut();
    }

    event
}
