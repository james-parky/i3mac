use crate::bits::EventFlags;
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

#[derive(Debug, PartialEq, Eq, Hash)]
enum Modifier {
    Command,
    Option,
    Shift,
    Control,
}

impl From<EventFlags> for Vec<Modifier> {
    fn from(value: EventFlags) -> Self {
        let mut ret = vec![];

        if value.has_command() {
            ret.push(Modifier::Command);
        }
        if value.has_alt() {
            ret.push(Modifier::Option);
        }
        if value.has_shift() {
            ret.push(Modifier::Shift);
        }
        if value.has_control() {
            ret.push(Modifier::Control);
        }

        ret
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct HotKey<'a> {
    modifiers: &'a [Modifier],
    pub key: Keycode,
}

impl<'a> HotKey<'a> {
    const fn new(modifiers: &'a [Modifier], key: Keycode) -> Self {
        Self { modifiers, key }
    }

    const RESIZE_LEFT: Self =
        Self::new(&[Modifier::Command, Modifier::Control], Keycode::LeftArrow);
    const RESIZE_RIGHT: Self =
        Self::new(&[Modifier::Command, Modifier::Control], Keycode::RightArrow);
    const RESIZE_UP: Self = Self::new(&[Modifier::Command, Modifier::Control], Keycode::UpArrow);
    const RESIZE_DOWN: Self =
        Self::new(&[Modifier::Command, Modifier::Control], Keycode::DownArrow);

    const OPEN_TERMINAL: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Return);

    const FOCUS_LEFT: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::LeftArrow);
    const FOCUS_RIGHT: Self =
        Self::new(&[Modifier::Command, Modifier::Option], Keycode::RightArrow);
    const FOCUS_UP: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::UpArrow);
    const FOCUS_DOWN: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::DownArrow);

    const MOVE_WINDOW_LEFT: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::LeftArrow,
    );
    const MOVE_WINDOW_RIGHT: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::RightArrow,
    );
    const MOVE_WINDOW_UP: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::UpArrow,
    );
    const MOVE_WINDOW_DOWN: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::DownArrow,
    );

    const FOCUS_DISPLAY_0: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi0);
    const FOCUS_DISPLAY_1: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi1);
    const FOCUS_DISPLAY_2: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi2);
    const FOCUS_DISPLAY_3: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi3);
    const FOCUS_DISPLAY_4: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi4);
    const FOCUS_DISPLAY_5: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi5);
    const FOCUS_DISPLAY_6: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi6);
    const FOCUS_DISPLAY_7: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi7);
    const FOCUS_DISPLAY_8: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi8);
    const FOCUS_DISPLAY_9: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::Ansi9);

    const MOVE_TO_DISPLAY_0: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi0,
    );
    const MOVE_TO_DISPLAY_1: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi1,
    );
    const MOVE_TO_DISPLAY_2: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi2,
    );
    const MOVE_TO_DISPLAY_3: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi3,
    );
    const MOVE_TO_DISPLAY_4: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi4,
    );
    const MOVE_TO_DISPLAY_5: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi5,
    );
    const MOVE_TO_DISPLAY_6: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi6,
    );
    const MOVE_TO_DISPLAY_7: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi7,
    );
    const MOVE_TO_DISPLAY_8: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi8,
    );
    const MOVE_TO_DISPLAY_9: Self = Self::new(
        &[Modifier::Command, Modifier::Option, Modifier::Shift],
        Keycode::Ansi9,
    );

    const VERTICAL_SPLIT: Self = Self::new(&[Modifier::Command, Modifier::Option], Keycode::AnsiV);
    const HORIZONTAL_SPLIT: Self =
        Self::new(&[Modifier::Command, Modifier::Option], Keycode::AnsiH);
}

#[derive(Debug, Hash, PartialEq, Eq)]
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
    ResizeWindow(Direction),
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

    let flags = unsafe { CGEventGetFlags(event) };
    let keycode_val = unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) };
    let keycode: Keycode = match keycode_val.try_into() {
        Ok(x) => x,
        Err(_) => return event,
    };

    let sender = unsafe { &*(user_info as *mut Sender<KeyCommand>) };
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

        _ => None,
    };

    if let Some(c) = command {
        let _ = sender.send(c);
        return std::ptr::null_mut();
    }

    event
}
