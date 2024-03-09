use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use late_mate_comms::{
    encode, CrcCobsAccumulator, DeviceToHost, FeedResult, HostToDevice, MAX_BUFFER_SIZE,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;
use tokio_serial::{
    SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream, UsbPortInfo,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stream current light level to console output (scaled to percents and throttled down to 120hz)
    MonitorBackground,
}

fn find_serial_port() -> anyhow::Result<SerialPortInfo> {
    tokio_serial::available_ports()
        .context("Serial port enumeration error")?
        .into_iter()
        .find(|info| match &info.port_type {
            SerialPortType::UsbPort(UsbPortInfo { manufacturer, .. }) => {
                manufacturer
                    .as_ref()
                    .is_some_and(|s| s.as_str() == "Late Mate")
                    && info.port_name.starts_with("/dev/cu.")
            }
            _ => false,
        })
        .ok_or(anyhow!("No appropriate serial port found"))
}

async fn device_loop(
    mut serial_stream: SerialStream,
    mut device_tx_receiver: mpsc::Receiver<HostToDevice>,
    device_rx_sender: broadcast::Sender<DeviceToHost>,
) -> anyhow::Result<()> {
    let mut cobs_acc = CrcCobsAccumulator::new();
    let mut usb_buf = [0u8; 64];
    let mut msg_buf = [0u8; MAX_BUFFER_SIZE];

    enum TxRx {
        Tx(HostToDevice),
        RxLen(usize),
    }

    loop {
        let tx_rx = tokio::select! {
            msg = device_tx_receiver.recv() => {
                msg.ok_or(anyhow!("Unexpectedly closed tx channel")).map(TxRx::Tx)
            },
            rx_len = serial_stream.read(&mut usb_buf) => {
                rx_len.context("Error reading the serial stream").map(TxRx::RxLen)
            }
        }?;

        match tx_rx {
            TxRx::Tx(msg) => {
                let msg_len = encode(&msg, &mut msg_buf);

                AsyncWriteExt::write_all(&mut serial_stream, &msg_buf[..msg_len]).await?;
            }
            TxRx::RxLen(rx_len) => {
                let mut window = &usb_buf[..rx_len];

                'cobs: while !window.is_empty() {
                    window = match cobs_acc.feed::<DeviceToHost>(window) {
                        FeedResult::Consumed => break 'cobs,
                        FeedResult::OverFull { .. } => {
                            return Err(anyhow!("USB buffer overflow"));
                        }
                        FeedResult::Error { error: e, .. } => {
                            return Err(anyhow!("Serial packet decoding failure ({e:?})"))
                        }
                        FeedResult::Success { data, remaining } => {
                            device_rx_sender.send(data)?;
                            remaining
                        }
                    }
                }
            }
        }
    }
}

// note: use single-threaded Tokio due to https://github.com/berkowski/tokio-serial/issues/69
//       or maybe just handle it carefully on a separate thread/runtime?
// note: https://github.com/berkowski/tokio-serial/issues/55
// note: https://github.com/berkowski/tokio-serial/issues/37
pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // todo: better device ID
    // todo: handle more than one device being connected

    let serial_port_info = find_serial_port()?;
    let serial_stream = tokio_serial::new(serial_port_info.port_name, 115200)
        .open_native_async()
        .context("Serial port opening failure")?;

    let (device_tx, device_tx_receiver) = mpsc::channel(4);
    let (device_rx_sender, device_rx) = broadcast::channel(1);

    let device_loop_future = device_loop(serial_stream, device_tx_receiver, device_rx_sender);

    let command_future = match &cli.command {
        Command::MonitorBackground => monitor_background(device_tx, device_rx),
    };

    tokio::select! {
        // device_loop can only return an error, but if it does we should stop trying to poll
        // the other future, it will fail anyway
        ret = device_loop_future => ret,
        ret = command_future => ret,
    }
}

pub async fn monitor_background(
    device_tx: mpsc::Sender<HostToDevice>,
    mut device_rx: broadcast::Receiver<DeviceToHost>,
) -> anyhow::Result<()> {
    let request_loop_future = async move {
        let mut interval = interval(Duration::from_millis(1000));

        loop {
            device_tx
                .send(HostToDevice::MeasureBackground { duration_ms: 1300 })
                .await
                .context("Error requesting more background light level values")?;
            interval.tick().await;
        }
    };

    let print_future = async move {
        // 120hz, no point streaming faster
        let mut interval = interval(Duration::from_millis(1000 / 120));
        loop {
            let msg = match device_rx.recv().await {
                Ok(x) => x,
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => {
                    return Err(anyhow!("Device RX channel was unexpectedly closed"))
                }
            };
            if let DeviceToHost::LightLevel { light_level, .. } = msg {
                println!(
                    "{:.4}",
                    (light_level as f64 / ((1 << 23) - 1) as f64) * 100f64
                )
            }
            interval.tick().await;
        }
    };

    tokio::select! {
        ret = request_loop_future => ret,
        ret = print_future => ret
    }
}
