use defmt_or_log::*;

use embassy_rp::peripherals::FLASH;
use static_cell::StaticCell;

pub struct SerialNumber {
    string: heapless::String<16>,
    bytes: [u8; 8],
}

impl SerialNumber {
    pub fn hex_str(&self) -> &str {
        self.string.as_str()
    }

    pub fn bytes(&self) -> [u8; 8] {
        self.bytes
    }
}

/// RP2040 doesn't have its own unique ID, but the datasheet suggests using
/// the flash chip's unique ID instead. Stores a heapless String in a static variable for ease
/// of access.
pub fn read(peripheral: FLASH) -> &'static SerialNumber {
    use core::fmt::Write;
    use embassy_rp::flash::{Blocking, Flash};

    static SERIAL_NUMBER: StaticCell<SerialNumber> = StaticCell::new();

    let mut flash = Flash::<_, Blocking, { crate::FLASH_SIZE }>::new_blocking(peripheral);

    let jedec_id = flash
        .blocking_jedec_id()
        .expect("JEDEC ID read must succeed");
    info!("Read a JEDEC ID: 0x{:X}", jedec_id);

    // JEDEC ID is a unique manufacturer ID. Apparently, not all flash manufacturers use unique IDs.
    // The link below claims Winbond is what Raspberry uses and that 0xEF7015 is their JEDEC ID,
    // but my device returns 0xEF4015, so I assert the latter just in case.
    // I expect this assert to turn into an allow list of JEDEC IDs.
    // See https://github.com/embassy-rs/embassy/blob/56a7b10064b830b1be1933085a5845d0d6be5f2e/embassy-rp/src/flash.rs#L668-L675
    self::assert_eq!(
        jedec_id,
        0xEF4015,
        "Expecting a flash chip with reliable unique ID"
    );

    let mut id_bytes = [0; 8];
    flash
        .blocking_unique_id(&mut id_bytes)
        .expect("Unique flash ID read must succeed");

    let mut id_str = heapless::String::new();
    id_bytes.iter().for_each(|b| {
        write!(&mut id_str, "{b:02X}")
            .expect("The string is appropriately sized and the write shouldn't fail")
    });

    let serial_number = SERIAL_NUMBER.init(SerialNumber {
        string: id_str,
        bytes: id_bytes,
    });

    info!("Read a unique flash ID: {}", serial_number.hex_str());

    serial_number
}
