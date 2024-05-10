use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::MutexKind;
use defmt::{debug, error, info};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Endpoint as RpEndpoint, In, Out};
use embassy_sync::channel::Channel;
use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
use embassy_usb::{msos, Builder};
use late_mate_shared::comms::usb_interface::ENDPOINT_INDEX;
use late_mate_shared::comms::{
    device_to_host, encode, host_to_device, usb_interface, CrcCobsAccumulator, FeedResult,
    MAX_BUFFER_SIZE as COMMS_MAX_BUFFER_SIZE,
};

// max number of serial in/out messages that can be buffered before waiting for more space
const FROM_HOST_N_BUFFERED: usize = 4;
const TO_HOST_N_BUFFERED: usize = 4;

static RX: Channel<MutexKind, host_to_device::Envelope, FROM_HOST_N_BUFFERED> = Channel::new();
static TX: Channel<MutexKind, device_to_host::Envelope, TO_HOST_N_BUFFERED> = Channel::new();

#[embassy_executor::task]
async fn rx_task(mut endpoint_out: RpEndpoint<'static, USB, Out>) {
    endpoint_out.wait_enabled().await;

    let mut cobs_acc = CrcCobsAccumulator::new();
    let mut usb_buf = [0u8; usb_interface::PACKET_SIZE];

    info!("Starting USB RX loop");
    loop {
        // todo: error handling
        let usb_len = endpoint_out.read(&mut usb_buf).await.unwrap();

        debug!("Received a USB packet ({} bytes)", usb_len);

        let mut window = &usb_buf[..usb_len];

        'cobs: while !window.is_empty() {
            window = match cobs_acc.feed::<host_to_device::Envelope>(window) {
                FeedResult::Consumed => {
                    debug!("The USB packet is fully consumed");
                    break 'cobs;
                }
                FeedResult::OverFull { remaining } => {
                    error!("COBS buffer is overfull");
                    remaining
                }
                FeedResult::Error {
                    error: e,
                    remaining,
                } => {
                    error!("COBS/CRC decoding error: {:?}", e);
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    debug!("The USB packet is decoded into {:?}", &data);
                    RX.send(data).await;
                    remaining
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn tx_task(mut endpoint_in: RpEndpoint<'static, USB, In>) {
    endpoint_in.wait_enabled().await;

    info!("Starting USB TX loop");
    loop {
        let msg = TX.receive().await;
        debug!("Received a new message to send to the host: {:?}", &msg);

        let buffer = &mut [0u8; COMMS_MAX_BUFFER_SIZE];
        // todo: encode shouldn't use .unwrap
        let packet_len = encode(&msg, buffer);

        debug!("The message is encoded ({} bytes)", packet_len);

        // todo: error handling
        match endpoint_in.write(&buffer[..packet_len]).await {
            Ok(()) => {}
            Err(e) => {
                error!("EndpointError sending to host: {:?}", e);
            }
        }
    }
}

pub async fn write_to_host(envelope: device_to_host::Envelope) {
    TX.send(envelope).await
}

pub async fn receive_from_host() -> host_to_device::Envelope {
    RX.receive().await
}

pub struct PreparedUsb {
    endpoint_in: RpEndpoint<'static, USB, In>,
    endpoint_out: RpEndpoint<'static, USB, Out>,
}

const _: () = assert!(
    usb_interface::PACKET_SIZE <= USB_MAX_PACKET_SIZE,
    "Bulk interface packets should fit into USB packets"
);

pub fn init_usb(
    builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
) -> PreparedUsb {
    static INTERFACE_GUIDS: &[&str] = &[usb_interface::GUID];

    // 0xFF function class is "vendor-specific"
    let mut function = builder.function(0xFF, 0, 0);

    // enable WinUSB on the function, this will allow driverless interaction with it on Windows
    function.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    function.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(INTERFACE_GUIDS),
    ));

    let mut interface = function.interface();

    let mut alt = interface.alt_setting(0xFF, 0, 0, None);

    assert_eq!(alt.interface_number().0, usb_interface::NUMBER);
    assert_eq!(alt.alt_setting_number(), usb_interface::ALT_SETTING_NUMBER);

    let endpoint_in = alt.endpoint_bulk_in(usb_interface::PACKET_SIZE as u16);
    let endpoint_out = alt.endpoint_bulk_out(usb_interface::PACKET_SIZE as u16);

    assert_eq!(endpoint_in.info().addr.index(), ENDPOINT_INDEX as usize);
    assert_eq!(endpoint_out.info().addr.index(), ENDPOINT_INDEX as usize);

    PreparedUsb {
        endpoint_in,
        endpoint_out,
    }
}

pub fn run(
    spawner: &Spawner,
    PreparedUsb {
        endpoint_in,
        endpoint_out,
    }: PreparedUsb,
) {
    spawner.must_spawn(rx_task(endpoint_out));
    spawner.must_spawn(tx_task(endpoint_in));
}
