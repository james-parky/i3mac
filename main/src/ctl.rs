use crate::config::Config;
use serde::{Deserialize, Serialize};

pub const CTL_SOCK: &str = "/tmp/i3mac/ctl.sock";

#[derive(Debug, Serialize, Deserialize)]
pub enum CtlToWmMessage {
    GetConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WmToCtlMessage {
    Config(Config),
}
