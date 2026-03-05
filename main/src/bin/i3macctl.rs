use main::ctl::{CTL_SOCK, CtlToWmMessage, WmToCtlMessage};
use std::io::Write;
use std::os::unix::net::UnixStream;

enum Mode {
    GetConfig,
}

enum OutputFormat {
    Json,
    Plain,
}

impl Mode {
    pub fn run(&self) -> Result<(), String> {
        let (msg, exp_resp) = match self {
            Mode::GetConfig => (CtlToWmMessage::GetConfig, true),
        };

        let mut tx = Vec::with_capacity(20);
        // derive serialize from serde for messages
        // msg.write_vec(tx)

        serde_json::to_writer(&mut tx, &msg).map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&tx).unwrap());

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

        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
        Ok(())
    }
}

fn main() {
    let mut args = std::env::args();
    let cmd = args.next().unwrap();

    let mut mode = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "get" => match args.next().unwrap().as_str() {
                "config" => mode = Some(Mode::GetConfig),
                _ => continue,
            },
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
