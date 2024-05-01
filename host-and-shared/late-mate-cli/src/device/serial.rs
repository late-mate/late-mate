use anyhow::{anyhow, Context};
use late_mate_shared::{USB_PID, USB_VID};
use tokio_serial::{SerialPortInfo, SerialPortType, UsbPortInfo};

pub fn find_serial_port() -> anyhow::Result<SerialPortInfo> {
    tokio_serial::available_ports()
        .context("Serial port enumeration error")?
        .into_iter()
        .find(|info| {
            // todo: this is, again, a hack (serial terminal doesn't tell me about itself on raspberry??)
            if info.port_name == "/dev/ttyACM0" {
                println!("found /dev/ttyACM0, shortcircuiting (A HACK)");
                return true;
            }

            if let SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) = &info.port_type {
                // todo: that starts_with is probably incorrect on Windows
                *vid == USB_VID
                    && *pid == USB_PID
                    && (info.port_name.starts_with("/dev/cu.")
                        // todo: this a hack to find the right serial terminal on linux
                        || info.port_name.starts_with("/dev/ttyACM"))
            } else {
                false
            }
        })
        .ok_or(anyhow!("No appropriate serial port found"))
}
