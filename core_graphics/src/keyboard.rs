use crate::{
    Error, Result,
    bits::{
        CGEventGetFlags, CGEventGetIntegerValueField, CGEventRef, CGEventTapCreate,
        CGEventTapEnable, CGEventTapProxy, EventTapLocation, EventTapOptions, EventTapPlacement,
        EventType, kCGKeyboardEventKeycode,
    },
};
use core_foundation::{
    CFMachPortCreateRunLoopSource, CFMachPortRef, CFRunLoopAddSource, CFRunLoopMode, CFRunLoopRef,
};
use std::{ffi::c_void, sync::mpsc::Sender};

pub enum Keycode {
    Return = 0x24,
    LeftArrow = 0x7B,
    RightArrow = 0x7C,
    DownArrow = 0x7D,
    UpArrow = 0x7E,
    AnsiQ = 0x0C,
    AnsiH = 0x04,
    AnsiV = 0x09,
    Ansi1 = 0x12,
    Ansi2 = 0x13,
    Ansi3 = 0x14,
    Ansi4 = 0x15,
    Ansi5 = 0x17,
    Ansi6 = 0x16,
    Ansi7 = 0x1A,
    Ansi8 = 0x1C,
    Ansi9 = 0x19,
    Ansi0 = 0x1D,
}

impl TryFrom<i64> for Keycode {
    type Error = ();

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0x24 => Ok(Keycode::Return),
            0x7B => Ok(Keycode::LeftArrow),
            0x7C => Ok(Keycode::RightArrow),
            0x7D => Ok(Keycode::DownArrow),
            0x7E => Ok(Keycode::UpArrow),
            0x0C => Ok(Keycode::AnsiQ),
            0x04 => Ok(Keycode::AnsiH),
            0x09 => Ok(Keycode::AnsiV),
            0x12 => Ok(Keycode::Ansi1),
            0x13 => Ok(Keycode::Ansi2),
            0x14 => Ok(Keycode::Ansi3),
            0x15 => Ok(Keycode::Ansi4),
            0x17 => Ok(Keycode::Ansi5),
            0x16 => Ok(Keycode::Ansi6),
            0x1A => Ok(Keycode::Ansi7),
            0x1C => Ok(Keycode::Ansi8),
            0x19 => Ok(Keycode::Ansi9),
            0x1D => Ok(Keycode::Ansi0),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug)]
pub enum KeyCommand {
    NewTerminal,
    CloseWindow,
    Focus(Direction),
    FocusDisplay(u64),
    MoveWindowToDisplay(u64),
    ToggleVerticalSplit,
    ToggleHorizontalSplit,
    MoveWindow(Direction),
}

pub struct KeyboardHandler {
    event_tap: CFMachPortRef,
}

impl KeyboardHandler {
    pub fn new(command_sender: Sender<KeyCommand>) -> Result<Self> {
        let tx_ptr = Box::into_raw(Box::new(command_sender));

        let event_mask = 1 << EventType::KeyDown;

        let event_tap = unsafe {
            CGEventTapCreate(
                EventTapLocation::Session,
                EventTapPlacement::HeadInsert,
                EventTapOptions::Default,
                event_mask,
                event_callback,
                tx_ptr as *mut c_void,
            )
        };

        if event_tap.is_null() {
            return Err(Error::FailedToCreateKeyboardEventTap);
        }

        unsafe { CGEventTapEnable(event_tap, true) };

        Ok(Self { event_tap })
    }

    pub fn add_to_run_loop(&self, run_loop: CFRunLoopRef, mode: CFRunLoopMode) {
        unsafe {
            let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), self.event_tap, 0);
            CFRunLoopAddSource(run_loop, source, mode);
        }
    }
}

extern "C" fn event_callback(
    _proxy: CGEventTapProxy,
    event_type: EventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    if event_type != EventType::KeyDown {
        return event;
    }

    let sender = unsafe { &*(user_info as *mut Sender<KeyCommand>) };
    let keycode: Keycode =
        match unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) }.try_into() {
            Ok(keycode) => keycode,
            Err(_) => return event,
        };

    let flags = unsafe { CGEventGetFlags(event) };

    let super_pressed = flags.has_alt() && flags.has_command();
    if !super_pressed {
        return event; // Not our hotkey
    }

    let command = match (keycode, flags.has_shift()) {
        // Super + Enter
        (Keycode::Return, false) => Some(KeyCommand::NewTerminal),

        // Super + Shift + Q
        (Keycode::AnsiQ, true) => Some(KeyCommand::CloseWindow),

        // Super + Arrow keys
        (Keycode::LeftArrow, false) => Some(KeyCommand::Focus(Direction::Left)),
        (Keycode::RightArrow, false) => Some(KeyCommand::Focus(Direction::Right)),
        (Keycode::UpArrow, false) => Some(KeyCommand::Focus(Direction::Up)),
        (Keycode::DownArrow, false) => Some(KeyCommand::Focus(Direction::Down)),

        (Keycode::LeftArrow, true) => Some(KeyCommand::MoveWindow(Direction::Left)),
        (Keycode::RightArrow, true) => Some(KeyCommand::MoveWindow(Direction::Right)),
        (Keycode::UpArrow, true) => Some(KeyCommand::MoveWindow(Direction::Up)),
        (Keycode::DownArrow, true) => Some(KeyCommand::MoveWindow(Direction::Down)),

        // Super + Number (focus display)
        (Keycode::Ansi1, false) => Some(KeyCommand::FocusDisplay(1)),
        (Keycode::Ansi2, false) => Some(KeyCommand::FocusDisplay(2)),
        (Keycode::Ansi3, false) => Some(KeyCommand::FocusDisplay(3)),
        (Keycode::Ansi4, false) => Some(KeyCommand::FocusDisplay(4)),
        (Keycode::Ansi5, false) => Some(KeyCommand::FocusDisplay(5)),
        (Keycode::Ansi6, false) => Some(KeyCommand::FocusDisplay(6)),
        (Keycode::Ansi7, false) => Some(KeyCommand::FocusDisplay(7)),
        (Keycode::Ansi8, false) => Some(KeyCommand::FocusDisplay(8)),
        (Keycode::Ansi9, false) => Some(KeyCommand::FocusDisplay(9)),
        (Keycode::Ansi0, false) => Some(KeyCommand::FocusDisplay(0)),

        // Super + Shift + Number (move to display)
        (Keycode::Ansi1, true) => Some(KeyCommand::MoveWindowToDisplay(1)),
        (Keycode::Ansi2, true) => Some(KeyCommand::MoveWindowToDisplay(2)),
        (Keycode::Ansi3, true) => Some(KeyCommand::MoveWindowToDisplay(3)),
        (Keycode::Ansi4, true) => Some(KeyCommand::MoveWindowToDisplay(4)),
        (Keycode::Ansi5, true) => Some(KeyCommand::MoveWindowToDisplay(5)),
        (Keycode::Ansi6, true) => Some(KeyCommand::MoveWindowToDisplay(6)),
        (Keycode::Ansi7, true) => Some(KeyCommand::MoveWindowToDisplay(7)),
        (Keycode::Ansi8, true) => Some(KeyCommand::MoveWindowToDisplay(8)),
        (Keycode::Ansi9, true) => Some(KeyCommand::MoveWindowToDisplay(9)),
        (Keycode::Ansi0, true) => Some(KeyCommand::MoveWindowToDisplay(0)),

        (Keycode::AnsiV, false) => Some(KeyCommand::ToggleVerticalSplit),
        (Keycode::AnsiH, false) => Some(KeyCommand::ToggleHorizontalSplit),

        _ => None,
    };

    if let Some(cmd) = command {
        println!("🎹 Key command: {:?}", cmd);
        let _ = sender.send(cmd);

        // Return null to consume the event (prevent default behavior)
        return std::ptr::null_mut();
    }

    // Pass through other events
    event
}
