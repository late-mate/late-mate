use crate::Error;
use late_mate_shared::comms::usb_interface;
use late_mate_shared::{comms, USB_PID, USB_VID};
use nusb::transfer;
use std::time::Duration;
use tokio::time::sleep;

// Nusb queue buffer is supposed to be a multiple of, so this is the size that will both
// fit postcard packets and also satisfy nusb requirements
pub const ALIGNED_BUFFER_SIZE: usize =
    (comms::MAX_BUFFER_SIZE / usb_interface::PACKET_SIZE + 1) * usb_interface::PACKET_SIZE;

pub struct UsbDevice {
    nusb_device: nusb::Device,
}

pub type InQueue = transfer::Queue<transfer::RequestBuffer>;
pub type OutQueue = transfer::Queue<Vec<u8>>;

impl UsbDevice {
    pub async fn acquire() -> Result<Self, Error> {
        let mut first_attempt = true;

        loop {
            let connected_devices = nusb::list_devices()
                .map_err(|e| Error::UsbError("listing devices", e))?
                .filter(|di| di.vendor_id() == USB_VID && di.product_id() == USB_PID)
                .collect::<Vec<_>>();

            if connected_devices.is_empty() {
                if first_attempt {
                    eprintln!("No Late Mate detected, waiting for the device to be connected");
                    first_attempt = false;
                }
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let first = &connected_devices[0];

            let n = connected_devices.len();
            if n > 1 {
                eprintln!(
                    "More than one Late Mate detected ({}), using {}",
                    n,
                    first
                        .serial_number()
                        .expect("Late Mate devices must have serial numbers")
                );
            }

            let nusb_device = first
                .open()
                .map_err(|e| Error::UsbError("opening the device", e))?;

            return Ok(Self { nusb_device });
        }
    }

    pub fn into_queues(self) -> Result<(InQueue, OutQueue), Error> {
        let interface = self
            .nusb_device
            .claim_interface(usb_interface::NUMBER)
            .map_err(|e| Error::UsbError("claiming the interface", e))?;

        let in_queue = interface.bulk_in_queue(usb_interface::ENDPOINT_INDEX | 0x80);
        let out_queue = interface.bulk_out_queue(usb_interface::ENDPOINT_INDEX);

        Ok((in_queue, out_queue))
    }
}
