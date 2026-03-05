use crate::log::Level;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub window_padding: Option<f64>,
    pub(crate) log_level: Level,
}

impl Config {
    pub fn must_parse() -> Self {
        let mut ret = Self::default();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--padding" => {
                    let padding = args
                        .next()
                        .expect("expected a usize value after --padding")
                        .parse::<usize>()
                        .expect("expected a usize value after --padding");
                    ret.window_padding = Some(padding as f64);
                }
                "--log-level" => {
                    let level: Level = args
                        .next()
                        .expect("expected one of {info, warn, error, trace}  after --log-level")
                        .as_str()
                        .try_into()
                        .expect("expected one of {info, warn, error, trace}  after --log-level");
                    ret.log_level = level;
                }
                unknown => {
                    panic!("{}", format!("unknown argument: {unknown}"));
                }
            }
        }

        ret
    }
}
