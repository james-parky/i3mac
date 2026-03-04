use serde::{Deserialize, Serialize};

pub const CTL_SOCK: &str = "/tmp/i3mac/ctl.sock";

#[derive(Debug, Serialize, Deserialize)]
pub enum CtlToWmMessage {
    GetConfigField(String),
    SetConfig(String, serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WmToCtlMessage {
    Value(Result<serde_json::Value, String>),
}
