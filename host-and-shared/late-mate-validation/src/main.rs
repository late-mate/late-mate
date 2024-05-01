use evdev::{EventType, InputEventKind, Key};
use std::error::Error;

use rppal::gpio::Gpio;
use rppal::system::DeviceInfo;

const GPIO_LED: u8 = 2;

// https://github.com/emberian/evdev/blob/main/examples/_pick_device.rs
pub fn pick_device() -> evdev::raw_stream::RawDevice {
    use std::io::prelude::*;

    let mut args = std::env::args_os();
    args.next();
    if let Some(dev_file) = args.next() {
        evdev::raw_stream::RawDevice::open(dev_file).unwrap()
    } else {
        let mut devices = evdev::raw_stream::enumerate()
            .map(|t| t.1)
            // only keyboards
            .filter(|d| {
                d.supported_keys()
                    .is_some_and(|keys| keys.contains(Key::KEY_A))
            })
            .filter(|d| d.name().is_some_and(|name| name.contains("Late Mate")))
            .collect::<Vec<_>>();
        // readdir returns them in reverse order from their eventN names for some reason
        devices.reverse();

        if devices.len() == 1 {
            devices.pop().unwrap()
        } else {
            for (i, d) in devices.iter().enumerate() {
                println!("{}: {}", i, d.name().unwrap_or("Unnamed device"));
            }
            print!("Select the device [0-{}]: ", devices.len());
            let _ = std::io::stdout().flush();
            let mut chosen = String::new();
            std::io::stdin().read_line(&mut chosen).unwrap();
            let n = chosen.trim().parse::<usize>().unwrap();
            devices.into_iter().nth(n).unwrap()
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "Running a validation helper on a {}, waiting for A to be pressed",
        DeviceInfo::new()?.model()
    );

    let mut d = pick_device();

    let mut pin = Gpio::new()?.get(GPIO_LED)?.into_output();
    pin.set_low();

    loop {
        for ev in d.fetch_events().unwrap() {
            if let InputEventKind::Key(Key::KEY_A) = ev.kind() {
                match ev.value() {
                    1 => {
                        pin.set_high();
                        println!("A is pressed, the LED is on");
                    }
                    0 => {
                        pin.set_low();
                        println!("A is released, the LED is off");
                    }
                    // this is just a repeat
                    2 => (),
                    _ => (),
                };
            }
        }
    }
}
