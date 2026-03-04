use main::ctl::{CTL_SOCK, CtlToWmMessage, WmToCtlMessage};
use std::io::Write;
use std::os::unix::net::UnixStream;

enum Mode {
    GetConfig(String),
    SetConfig(String, serde_json::Value),
}

impl Mode {
    pub fn run(&self) -> Result<(), String> {
        let (msg, exp_resp) = match self {
            Mode::GetConfig(index) => (CtlToWmMessage::GetConfigField(index.clone()), true),
            Mode::SetConfig(index, value) => (
                CtlToWmMessage::SetConfig(index.clone(), value.clone()),
                false,
            ),
        };

        let mut tx = Vec::with_capacity(20);

        serde_json::to_writer(&mut tx, &msg).map_err(|e| e.to_string())?;

        let mut stream = UnixStream::connect(CTL_SOCK).unwrap();
        stream.write_all(&tx).map_err(|e| e.to_string())?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| e.to_string())?;

        if !exp_resp {
            return Ok(());
        }

        let msg: WmToCtlMessage =
            serde_json::from_reader(&mut stream).map_err(|e| e.to_string())?;

        match msg {
            WmToCtlMessage::Value(Ok(val)) => {
                println!("{}", serde_json::to_string_pretty(&val).unwrap());
                Ok(())
            }
            WmToCtlMessage::Value(Err(e)) => Err(e),
        }
    }
}

fn main() {
    let mut args = std::env::args();
    let cmd = args.next().unwrap();

    let mut mode = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "get" => {
                let index = args.next().unwrap();
                if index.starts_with("config.") {
                    mode = Some(Mode::GetConfig(
                        index.strip_prefix("config.").unwrap().to_string(),
                    ));
                }
            }

            "set" => {
                let index = args.next().unwrap();
                let value = args.next().unwrap();
                if index.starts_with("config.") {
                    mode = Some(Mode::SetConfig(
                        index.strip_prefix("config.").unwrap().to_string(),
                        serde_json::to_value(value).unwrap(),
                    ));
                }
            }
            _ => continue,
        }
    }

    if let Some(mode) = mode {
        if let Err(err) = mode.run() {
            eprintln!("{}", err);
            std::process::exit(2);
        }
    } else {
        println!("usage: {cmd} <mode> [opts...]");
        std::process::exit(1);
    }
}
