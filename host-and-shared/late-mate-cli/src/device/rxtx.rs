use late_mate_shared::comms;
use late_mate_shared::comms::usb_interface;

pub mod rx;
pub mod tx;

// Nusb queue buffer is supposed to be a multiple of, so this is the size that will both
// fit postcard packets and also satisfy nusb requirements
const ALIGNED_BUFFER_SIZE: usize =
    (comms::MAX_BUFFER_SIZE / usb_interface::PACKET_SIZE + 1) * usb_interface::PACKET_SIZE;
