use std::error::Error;

use crate::backend::{client::FenderClient, device::{self, FenderDevice}};

pub type DynamicError = Box<dyn Error>;

mod backend;
mod amp;
mod cli;

fn main() -> Result<(), DynamicError> {
    let client = FenderClient::new(
        FenderDevice::open(
            &device::new_api_instance()?
        )?
    );

    client.sync_begin()?;
    client.get_firmware()?;
    let preset = client.get_preset(01)?;
    println!("{preset}");
    client.sync_end()?;
    client.hearbeat()?;
    
    client.set_preset_by_index(02)?;

    Ok(())
}


// todo :: make genuine tests
#[cfg(test)]
mod tests {
    use crate::{backend::client::FenderClient, device::{self, FenderDevice}};

    #[test]
    #[ignore]
    fn lt25_sync_round_trip() {
        let client = FenderClient::new(FenderDevice::open(&device::new_api_instance().expect("new api")).expect("opening of device"));

        client.sync_begin().expect("sent sync_begin");

    }
}
