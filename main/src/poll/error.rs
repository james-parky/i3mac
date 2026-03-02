#[derive(Debug)]
pub enum Error {
    FailedToCreateRunLoopSource,
    FailedToCreateKeyboardEventTap,
    FailedToCreateMux,
}
