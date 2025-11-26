use crate::bits::EventFlags;
use crate::{
    Error, Result,
    bits::{
        CGEventGetFlags, CGEventGetIntegerValueField, CGEventRef, CGEventTapCreate,
        CGEventTapEnable, CGEventTapProxy, EventTapLocation, EventTapOptions, EventTapPlacement,
        EventType, KEYBOARD_EVENT_KEYCODE,
    },
};
use core_foundation::{
    CFMachPortCreateRunLoopSource, CFMachPortRef, CFRelease, CFRunLoopAddSource, CFRunLoopMode,
    CFRunLoopRef, CFTypeRef,
};
use std::fmt::Display;
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

impl Modifier {
    const CMD_CTRL: [Self; 2] = [Self::Command, Self::Control];
    const CMD_OPTN: [Self; 2] = [Self::Command, Self::Option];
    const CMD_OPTN_SHFT: [Self; 3] = [Self::Command, Self::Option, Self::Shift];
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

    // TODO: use macros?
    const RESIZE_LEFT: Self = Self::new(&Modifier::CMD_CTRL, Keycode::LeftArrow);
    const RESIZE_RIGHT: Self = Self::new(&Modifier::CMD_CTRL, Keycode::RightArrow);
    const RESIZE_UP: Self = Self::new(&Modifier::CMD_CTRL, Keycode::UpArrow);
    const RESIZE_DOWN: Self = Self::new(&Modifier::CMD_CTRL, Keycode::DownArrow);

    const OPEN_TERMINAL: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Return);

    const FOCUS_LEFT: Self = Self::new(&Modifier::CMD_OPTN, Keycode::LeftArrow);
    const FOCUS_RIGHT: Self = Self::new(&Modifier::CMD_OPTN, Keycode::RightArrow);
    const FOCUS_UP: Self = Self::new(&Modifier::CMD_OPTN, Keycode::UpArrow);
    const FOCUS_DOWN: Self = Self::new(&Modifier::CMD_OPTN, Keycode::DownArrow);

    const MOVE_WINDOW_LEFT: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::LeftArrow);
    const MOVE_WINDOW_RIGHT: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::RightArrow);
    const MOVE_WINDOW_UP: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::UpArrow);
    const MOVE_WINDOW_DOWN: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::DownArrow);

    const FOCUS_DISPLAY_0: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi0);
    const FOCUS_DISPLAY_1: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi1);
    const FOCUS_DISPLAY_2: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi2);
    const FOCUS_DISPLAY_3: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi3);
    const FOCUS_DISPLAY_4: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi4);
    const FOCUS_DISPLAY_5: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi5);
    const FOCUS_DISPLAY_6: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi6);
    const FOCUS_DISPLAY_7: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi7);
    const FOCUS_DISPLAY_8: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi8);
    const FOCUS_DISPLAY_9: Self = Self::new(&Modifier::CMD_OPTN, Keycode::Ansi9);

    const MOVE_TO_DISPLAY_0: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi0);
    const MOVE_TO_DISPLAY_1: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi1);
    const MOVE_TO_DISPLAY_2: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi2);
    const MOVE_TO_DISPLAY_3: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi3);
    const MOVE_TO_DISPLAY_4: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi4);
    const MOVE_TO_DISPLAY_5: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi5);
    const MOVE_TO_DISPLAY_6: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi6);
    const MOVE_TO_DISPLAY_7: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi7);
    const MOVE_TO_DISPLAY_8: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi8);
    const MOVE_TO_DISPLAY_9: Self = Self::new(&Modifier::CMD_OPTN_SHFT, Keycode::Ansi9);

    const VERTICAL_SPLIT: Self = Self::new(&Modifier::CMD_OPTN, Keycode::AnsiV);
    const HORIZONTAL_SPLIT: Self = Self::new(&Modifier::CMD_OPTN, Keycode::AnsiH);

    const TOGGLE_FLOATING: Self = Self::new(&Modifier::CMD_OPTN, Keycode::AnsiC);
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
    AnsiC = 0x08,
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
            0x08 => Ok(Keycode::AnsiC),
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

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Left => f.write_str("Left"),
            Direction::Right => f.write_str("Right"),
            Direction::Up => f.write_str("Up"),
            Direction::Down => f.write_str("Down"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum KeyCommand {
    NewTerminal,
    CloseWindow,
    Focus(Direction),
    FocusDisplay(usize),
    MoveWindowToDisplay(u64),
    ToggleVerticalSplit,
    ToggleHorizontalSplit,
    MoveWindow(Direction),
    ResizeWindow(Direction),
    ToggleFloating,
}

pub struct KeyboardHandler {
    event_tap: CFMachPortRef,
    tx_ptr: *mut Sender<KeyCommand>,
}

impl Drop for KeyboardHandler {
    fn drop(&mut self) {
        unsafe {
            CGEventTapEnable(self.event_tap, false);
            CFRelease(CFTypeRef(self.event_tap));
            let _ = Box::from_raw(self.tx_ptr);
        }
    }
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
            unsafe {
                let _ = Box::from_raw(tx_ptr);
            }
            return Err(Error::FailedToCreateKeyboardEventTap);
        }

        unsafe { CGEventTapEnable(event_tap, true) };

        Ok(Self { event_tap, tx_ptr })
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
    let keycode_val = unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) };
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

        HotKey::TOGGLE_FLOATING => Some(KeyCommand::ToggleFloating),

        _ => None,
    };

    if let Some(c) = command {
        let _ = sender.send(c);
        return std::ptr::null_mut();
    }

    event
}
