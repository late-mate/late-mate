mod cli;

use anyhow::Context;
use late_mate_device::Device;
use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn run() -> anyhow::Result<()> {
    let parsed_cli: cli::Cli = clap::Parser::parse();

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::debug!("Initialising the device");
    let mut device = Device::init().await?;

    tracing::debug!("Running the command");
    parsed_cli.command.run(&mut device).await
}

// pub async fn monitor_background(mut device: Device) -> anyhow::Result<()> {
//     let mut receiver = device.subscribe_to_background();
//     device.background_enable();
//
//     // 120hz, no point streaming faster
//     let mut interval = interval(Duration::from_millis(1000 / 120));
//     loop {
//         match receiver.recv().await {
//             Ok(light_level) => {
//                 println!(
//                     "{:.4}",
//                     (light_level as f64 / device.max_light_level as f64) * 100f64
//                 )
//             }
//             Err(RecvError::Lagged(_)) => continue,
//             Err(RecvError::Closed) => return Err(anyhow!("Background light level channel closed")),
//         };
//         interval.tick().await;
//     }
// }

// pub async fn hid_demo(
//     device_tx: mpsc::Sender<HostToDevice>,
//     mut device_rx: broadcast::Receiver<DeviceToHost>,
//     csv_filename: String,
// ) -> anyhow::Result<()> {
//     sleep(Duration::from_secs(3)).await;
//
//     let req_future = async move {
//         device_tx
//             .send(HostToDevice::SendHidEvent {
//                 hid_event: HidReport::Keyboard(KeyboardReport {
//                     modifier: 0,
//                     reserved: 0,
//                     leds: 0,
//                     keycodes: [KeyboardUsage::KeyboardAa as u8, 0, 0, 0, 0, 0],
//                 }),
//                 duration_ms: 300,
//             })
//             .await
//             .context("Device TX channel was unexpectedly closed")
//     };
//     let resp_future = async move {
//         let mut data: Vec<(&'static str, u64, u32)> = vec![];
//         loop {
//             // todo: make this sane
//             match timeout(Duration::from_millis(100), device_rx.recv()).await {
//                 Ok(Ok(DeviceToHost::LightLevel {
//                     microsecond,
//                     light_level,
//                 })) => data.push(("light_level", microsecond, light_level)),
//                 Ok(Ok(DeviceToHost::HidReport { microsecond, .. })) => {
//                     data.push(("hid_event", microsecond, 0))
//                 }
//                 Ok(Ok(_)) => continue,
//                 Ok(Err(RecvError::Lagged(_))) => continue,
//                 Ok(Err(RecvError::Closed)) => {
//                     let result: anyhow::Result<Vec<(&'static str, u64, u32)>> =
//                         Err(anyhow!("Device RX channel was unexpectedly closed"));
//                     return result;
//                 }
//                 Err(_) => return Ok(data),
//             }
//         }
//     };
//
//     match tokio::try_join!(req_future, resp_future) {
//         Ok((_, data)) => {
//             dbg!(&data);
//             let mut file = File::create(csv_filename).context("CSV file creation error")?;
//             let mut start: Option<u64> = None;
//             for (kind, microsecond, light_level) in data {
//                 match start {
//                     None if kind == "hid_event" => start = Some(microsecond),
//                     None => continue,
//                     Some(start_microsecond) => {
//                         writeln!(
//                             file,
//                             "{:.4},{:.4}",
//                             (microsecond - start_microsecond) as f64 / 1000f64,
//                             light_level as f64 / ((1 << 23) - 1) as f64
//                         )
//                         .context("CSV file write error")?;
//                     }
//                 }
//             }
//             file.flush()?;
//             Ok(())
//         }
//         // have to rewrap to change the type ('Ok' branches are different)
//         Err(e) => Err(e),
//     }
// }
