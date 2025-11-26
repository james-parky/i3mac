use crate::display::LogicalDisplayId;
use core_graphics::{Direction, DisplayId, KeyCommand, WindowId};
use std::fmt::format;
use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Default, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Level {
    #[default]
    Trace,
    Info,
    Warn,
    Error,
}

impl TryFrom<&str> for Level {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        match value {
            "info" => Ok(Level::Info),
            "warn" => Ok(Level::Warn),
            "error" => Ok(Level::Error),
            "trace" => Ok(Level::Trace),
            _ => Err(()),
        }
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => f.write_str("INFO"),
            Self::Warn => f.write_str("WARN"),
            Self::Error => f.write_str("ERROR"),
            Self::Trace => f.write_str("TRACE"),
        }
    }
}

pub trait Log {
    fn prefix(&self) -> String;
    fn level(&self) -> Level;
    fn message(&self) -> String;
    fn log(&self, logger: &mut Logger) {
        logger.log(self.level(), &self.prefix(), &self.message())
    }
}

pub struct Logger {
    file: File,
    level: Level,
}

impl Logger {
    pub fn try_new<P: AsRef<Path>>(path: P, level: Level) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file, level })
    }

    pub fn log(&mut self, level: Level, prefix: &str, message: &str) {
        if level >= self.level {
            // TODO: what to do about errors logging? default to stdout/stderr print?
            let _ = self
                .file
                .write_all(format!("[{}] : {} : {}\n", level, prefix, message).as_bytes());
        }
    }
}

pub enum Message {
    ReceivedWindowAddedEvent(DisplayId, WindowId),
    ReceivedWindowRemovedEvent(DisplayId, WindowId),
    ReceivedWindowFocusedEvent(WindowId),
    ReceivedKeyCommand(KeyCommand),

    WindowAdded(DisplayId, LogicalDisplayId, WindowId),
    WindowRemoved(DisplayId, LogicalDisplayId, WindowId),
    WindowFocused(WindowId),

    OpenTerminalKeyCommand,
    // ClosedWindowKeyCommand(WindowId),
    ShiftFocusInDirectionKeyCommand(Direction),
    FocusLogicalDisplayKeyCommand(LogicalDisplayId),
    MoveFocusedWindowToLogicalDisplayKeyCommand(LogicalDisplayId),
    ToggleVerticalSplitKeyCommand,
    ToggleHorizontalSplitKeyCommand,
    ResizeWindowInDirectionKeyCommand(Direction),
    ToggleWindowFloatingKeyCommand,
}

impl Log for Message {
    fn prefix(&self) -> String {
        "WM".into()
    }

    fn level(&self) -> Level {
        match self {
            Message::ReceivedWindowAddedEvent(_, _) => Level::Trace,
            Message::ReceivedWindowRemovedEvent(_, _) => Level::Trace,
            Message::ReceivedWindowFocusedEvent(_) => Level::Trace,
            Message::ReceivedKeyCommand(_) => Level::Trace,

            Message::WindowAdded(_, _, _) => Level::Info,
            Message::WindowRemoved(_, _, _) => Level::Info,
            Message::WindowFocused(_) => Level::Info,

            Message::OpenTerminalKeyCommand => Level::Trace,
            Message::ShiftFocusInDirectionKeyCommand(_) => Level::Trace,
            Message::FocusLogicalDisplayKeyCommand(_) => Level::Trace,
            Message::MoveFocusedWindowToLogicalDisplayKeyCommand(_) => Level::Trace,
            Message::ToggleVerticalSplitKeyCommand => Level::Trace,
            Message::ToggleHorizontalSplitKeyCommand => Level::Trace,
            Message::ResizeWindowInDirectionKeyCommand(_) => Level::Trace,
            Message::ToggleWindowFloatingKeyCommand => Level::Trace,
        }
    }

    fn message(&self) -> String {
        use Message::*;

        match self {
            ReceivedWindowAddedEvent(d_id, w_id) => {
                format!("window add event received for {w_id} on display {d_id}")
            }
            ReceivedWindowRemovedEvent(d_id, w_id) => {
                format!("window removed event received for {w_id} on display {d_id}")
            }
            ReceivedWindowFocusedEvent(w_id) => {
                format!("received window focus event for window {w_id}")
            }
            ReceivedKeyCommand(kc) => format!("keyboard command input received {kc:?}"),

            WindowAdded(p_id, l_id, w_id) => format!("added window {w_id} to {p_id}{l_id}"),
            WindowRemoved(p_id, l_id, w_id) => {
                format!("removed window {w_id} from {p_id}{l_id}")
            }
            WindowFocused(w_id) => format!("focused window {w_id}"),

            OpenTerminalKeyCommand => "open terminal key command input received".into(),
            ShiftFocusInDirectionKeyCommand(d) => {
                format!("shift focus {d} key command input received")
            }
            FocusLogicalDisplayKeyCommand(l_id) => {
                format!("focus {l_id} key command input received")
            }
            MoveFocusedWindowToLogicalDisplayKeyCommand(l_id) => {
                format!("move focused window to {l_id} key command input received")
            }
            ToggleVerticalSplitKeyCommand => {
                "toggle vertical split key command input received".into()
            }
            ToggleHorizontalSplitKeyCommand => {
                "toggle horizontal split key command input received".into()
            }
            ResizeWindowInDirectionKeyCommand(d) => {
                format!("resize window {d} key command input received")
            }
            ToggleWindowFloatingKeyCommand => {
                "toggle window floating key command input received".into()
            }
        }
    }
}
