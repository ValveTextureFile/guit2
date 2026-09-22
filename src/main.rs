use std::{
    error::Error,
    io::{self, Write},
};

use crate::{amp::values::Preset, backend::{
    client::FenderClient,
    device::{self, FenderDevice},
}};

use colored::Colorize;

pub type DynamicError = Box<dyn Error>;

mod amp;
mod backend;

fn command_loop(client: &FenderClient) -> Result<(), DynamicError> {
    loop {
        print!("{}", "guit2 ~> ".green());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let mut args = input.split_whitespace();

        let Some(cmd) = args.next() else {
            continue;
        };

        match cmd {
            "help" => {
                println!("guit2 commands | list as of 2026-09-21");
                println!("\tversion ~ get guit2 version");
                println!("\tfirmware ~ print firmware version to screen");
                println!("\tpreset [1..60] ~ prints current preset or switches to i32 provided");
                println!("\tquit ~ quit.")
            }

            "version" => {
                println!("guit2 {}, made with rust {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_RUST_VERSION"))
            }

            "firmware" => {
                println!("connected to amp with firmware version {}", client.get_firmware()?)
            }

            "preset" => {
                match args.next() {
                    Some(id) => {
                        let id: i32 = id.parse()?;

                        client.set_preset_by_index(id)?;

                        println!("loaded preset {id}")
                    }

                    None => {
                        let preset = Preset::from_data(client.get_current_preset()?.current_preset_data)?;

                        println!("loaded preset is {:?}", preset.info.display_name)
                    }
                }
            }

            "quit" => {
                client.fuckoff();
                println!("broke worker thread");
                break Ok(());
            }

            unknown => {
                println!("unknown command :: {unknown}");
            }
        }
    }
}

fn main() -> Result<(), DynamicError> {
    // check if running root ~> root is required in order to make communications with hidraw*
    if !std::env::var("SUDO_USER").is_ok() {
        return Err("you must be root or run with sudo inorder to run guit2.".into());
    }

    let client: FenderClient = FenderClient::new(FenderDevice::open(&device::new_api_instance()?)?);

    client.sync_begin()?;
    client.get_firmware()?;
    client.sync_end()?;
    client.hearbeat()?;

    command_loop(&client)?;

    Ok(())
}

// todo :: make genuine tests
#[cfg(test)]
mod tests {
    use crate::{
        backend::client::FenderClient,
        device::{self, FenderDevice},
    };

    #[test]
    #[ignore]
    fn lt25_sync_round_trip() {
        let client = FenderClient::new(
            FenderDevice::open(&device::new_api_instance().expect("new api"))
                .expect("opening of device"),
        );

        client.sync_begin().expect("sent sync_begin");
    }
}
