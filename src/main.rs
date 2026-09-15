use std::error::Error;

use crate::{
    client::FenderClient, device::FenderDevice, protocol::LTMessage,
};

pub type DynamicError = Box<dyn Error>;

mod client;
mod device;
mod fenderprotos;
mod protocol;
mod report;

fn main() -> Result<(), DynamicError> {
    // let mut client = FenderClient::new(FenderDevice::open(&device::new_api_instance()?)?);
    //! implement threading
    // this is currently under construction due to threading implemenetation

    // start sync
    client.sync_begin()?;
    println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~>");
    client.sync_end()?;
    println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~>");
    // client.hearbeat()?;    
    client.hearbeat()?;

    

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{client::FenderClient, device::{self, FenderDevice}};

    #[test]
    #[ignore]
    fn lt25_sync_round_trip() {
        let mut client = FenderClient::new(FenderDevice::open(&device::new_api_instance().expect("new api")).expect("opening of device"));

        client.sync_begin().expect("sent sync_begin");

    }
}
