use anyhow::{anyhow, Context};
use late_mate_comms::{USB_PID, USB_VID};
use tokio_serial::{SerialPortInfo, SerialPortType, UsbPortInfo};

pub fn find_serial_port() -> anyhow::Result<SerialPortInfo> {
    tokio_serial::available_ports()
        .context("Serial port enumeration error")?
        .into_iter()
        .find(|info| {
            if let SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) = &info.port_type {
                // todo: that starts_with is probably incorrect on Windows
                *vid == USB_VID && *pid == USB_PID && info.port_name.starts_with("/dev/cu.")
            } else {
                false
            }
        })
        .ok_or(anyhow!("No appropriate serial port found"))
}
