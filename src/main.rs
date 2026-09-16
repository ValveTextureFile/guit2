use std::error::Error;

use crate::backend::{client::FenderClient, device::{self, FenderDevice}};

pub type DynamicError = Box<dyn Error>;

mod backend;

fn main() -> Result<(), DynamicError> {
    let client = FenderClient::new(
        FenderDevice::open(
            &device::new_api_instance()?
        )?
    );

    client.sync_begin()?;
    client.get_firmware()?;
    client.sync_end()?;
    client.hearbeat()?;
    

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
