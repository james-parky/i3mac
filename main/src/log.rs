use crate::{container::Axis, display::LogicalDisplayId};
use core_graphics::{Direction, DisplayId, KeyCommand, WindowId};
use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
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
            let _ = self.file.write_all(
                format!(
                    "{} i3mac {} [{}] : {} : {}\n",
                    current_time(),
                    std::process::id(),
                    level,
                    prefix,
                    message
                )
                .as_bytes(),
            );
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
    WindowMadeFloating(WindowId),
    WindowMadeManaged(WindowId),
    WindowResized(WindowId, Direction),
    WindowSplitAlongAxis(WindowId, Axis),
    ShiftedFocusInDirection(Direction),
    WindowMovedToLogicalDisplay(WindowId, LogicalDisplayId),
    FocusedLogicalDisplay(LogicalDisplayId),

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
        use Message::*;

        match self {
            ReceivedWindowAddedEvent(_, _) => Level::Trace,
            ReceivedWindowRemovedEvent(_, _) => Level::Trace,
            ReceivedWindowFocusedEvent(_) => Level::Trace,
            ReceivedKeyCommand(_) => Level::Trace,

            WindowAdded(_, _, _) => Level::Info,
            WindowRemoved(_, _, _) => Level::Info,
            WindowFocused(_) => Level::Info,
            WindowMadeFloating(_) => Level::Info,
            WindowMadeManaged(_) => Level::Info,
            WindowResized(_, _) => Level::Info,
            WindowSplitAlongAxis(_, _) => Level::Info,
            ShiftedFocusInDirection(_) => Level::Info,
            WindowMovedToLogicalDisplay(_, _) => Level::Info,
            FocusedLogicalDisplay(_) => Level::Info,

            OpenTerminalKeyCommand => Level::Trace,
            ShiftFocusInDirectionKeyCommand(_) => Level::Trace,
            FocusLogicalDisplayKeyCommand(_) => Level::Trace,
            MoveFocusedWindowToLogicalDisplayKeyCommand(_) => Level::Trace,
            ToggleVerticalSplitKeyCommand => Level::Trace,
            ToggleHorizontalSplitKeyCommand => Level::Trace,
            ResizeWindowInDirectionKeyCommand(_) => Level::Trace,
            ToggleWindowFloatingKeyCommand => Level::Trace,
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
            WindowMadeFloating(w_id) => format!("toggle window {w_id} as floating"),
            WindowMadeManaged(w_id) => format!("toggle window {w_id} as managed"),
            WindowResized(w_id, d) => format!("resized window {w_id} {d}"),
            WindowSplitAlongAxis(w_id, d) => format!("split window {w_id} along {d} axis"),
            ShiftedFocusInDirection(d) => format!("shifted focus {d}"),
            WindowMovedToLogicalDisplay(w_id, l_id) => format!("moved window {w_id} to {l_id}"),
            FocusedLogicalDisplay(l_id) => format!("focused {l_id}"),

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

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: i32) -> i64 {
    match month {
        1 => 31,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => unreachable!(),
    }
}

fn unix_to_utc(ts: u64) -> (i32, i32, i32, i32, i32, i32) {
    let mut seconds = ts as i64;

    let sec = (seconds % 60) as i32;
    seconds /= 60;
    let min = (seconds % 60) as i32;
    seconds /= 60;
    let hour = (seconds % 24) as i32;
    let mut days = seconds / 24;

    let mut year = 1970;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days >= dy {
            days -= dy;
            year += 1;
        } else {
            break;
        }
    }

    let mut month = 1;
    loop {
        let dm = days_in_month(year, month);
        if days >= dm {
            days -= dm;
            month += 1;
        } else {
            break;
        }
    }

    let day = days as i32 + 1;

    (year, month, day, hour, min, sec)
}

pub fn current_time() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let seconds = now.as_secs();
    let millis = now.subsec_millis();
    let (y, m, d, h, mi, s) = unix_to_utc(seconds);

    format!("{y:04}-{m:02}-{d:02}:{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}
