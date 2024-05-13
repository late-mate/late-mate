use anyhow::Context;
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
    pub async fn acquire() -> anyhow::Result<Self> {
        let mut first_attempt = true;

        loop {
            let connected_devices = nusb::list_devices()
                .context("USB error while listing devices")?
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

            let nusb_device = first.open().context("USB error while opening the device")?;

            return Ok(Self { nusb_device });
        }
    }

    pub fn into_queues(self) -> anyhow::Result<(InQueue, OutQueue)> {
        let interface = self
            .nusb_device
            .claim_interface(usb_interface::NUMBER)
            .context("USB error while claiming the interface")?;

        let in_queue = interface.bulk_in_queue(usb_interface::ENDPOINT_INDEX | 0x80);
        let out_queue = interface.bulk_out_queue(usb_interface::ENDPOINT_INDEX);

        Ok((in_queue, out_queue))
    }
}
