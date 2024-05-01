pub mod api;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

use anyhow::Context;
use std::future::Future;
use std::net::IpAddr;
use std::sync::Arc;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::extract::connect_info::ConnectInfo;

use crate::device::Device;
use futures::{sink::SinkExt, stream::StreamExt};
use late_mate_shared::Measurement;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{mpsc, watch, Mutex as TokioMutex};
use tokio::task::JoinHandle;

#[derive(Clone)]
struct ServerState {
    device: Arc<TokioMutex<Device>>,
}

pub async fn run(device: Device, interface: IpAddr, port: u16) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "late-mate-cli=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    // build our application with some routes
    let app = Router::new()
        //.fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler));

    let app = app
        // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(ServerState {
            device: Arc::new(TokioMutex::new(device)),
        }));

    let socket_addr = SocketAddr::from((interface, port));
    let listener = tokio::net::TcpListener::bind(&socket_addr)
        .await
        .context(format!("Couldn't bind on {socket_addr}"))?;
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("Axum error")
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    server_state: Extension<ServerState>,
) -> impl IntoResponse {
    println!("{addr} connected");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| log_error(handle_socket(socket, addr, server_state.device.clone())))
}

async fn log_error(result_fut: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
    if let Err(e) = result_fut.await {
        eprintln!("{e:?}");
    }
}

// spawned per connection
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    device: Arc<TokioMutex<Device>>,
) -> anyhow::Result<()> {
    socket
        .send(Message::Ping(vec![1, 3, 3, 7]))
        .await
        .context(format!("Could not send ping to {who}"))?;
    println!("Pinged {who}");

    let (bg_streaming_enabled_sender, mut bg_streaming_enabled_receiver) = watch::channel(false);
    let mut bg_receiver = device.lock().await.subscribe_to_background();

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (to_client_sender, mut to_client_receiver) = mpsc::channel::<api::ServerToClient>(4);

    let device_status = device.lock().await.get_status().await?;
    let max_light_level = device_status.max_light_level;

    let mut bg_light_task: JoinHandle<anyhow::Result<()>> = tokio::spawn({
        let device = device.clone();
        let to_client_sender = to_client_sender.clone();
        async move {
            // 2 values per ms; 60 fps = 16.6ms/frame; 40 samples per buffer (=50hz) should be OK
            let buffer_size = 40;
            let mut buffer: Vec<u32> = Vec::with_capacity(buffer_size);
            loop {
                let enabled = *bg_streaming_enabled_receiver.borrow_and_update();
                if !enabled {
                    device.lock().await.background_disable();
                    buffer.clear();
                    if dbg!(bg_streaming_enabled_receiver.changed().await).is_ok() {
                        continue;
                    } else {
                        println!("all bg_streaming_enabled_sender dropped");
                        return Ok(());
                    }
                }
                device.lock().await.background_enable();
                let light_level = match bg_receiver.recv().await {
                    Ok(x) => x,
                    Err(RecvError::Lagged(_)) => continue,
                    e @ Err(_) => e.context("bg_receiver has closed")?,
                };
                if buffer.len() == buffer_size {
                    let avg_light_level =
                        buffer.iter().map(|x| *x as f64).sum::<f64>() / buffer.len() as f64;
                    let fraction = avg_light_level / max_light_level as f64;
                    to_client_sender
                        .send(api::ServerToClient::BackgroundLightLevel { avg: fraction })
                        .await?;
                    buffer.clear();
                }
                buffer.push(light_level);
            }
        }
    });

    let mut send_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(msg) = to_client_receiver.recv().await {
            ws_sender
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await?;
        }
        Ok(())
    });

    // todo: recv_task errors (e.g. validation error, a mouse value too big to fit into i8)
    //       somehow get swallowed, investigate
    let mut recv_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let bg_streaming_enabled_sender = bg_streaming_enabled_sender;
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(txt) => {
                    let from_client: api::ClientToServer = serde_json::from_str(&txt)?;
                    handle_message(
                        from_client,
                        device.as_ref(),
                        &to_client_sender,
                        &bg_streaming_enabled_sender,
                    )
                    .await?;
                }
                Message::Close(_) => return Ok(()),
                // shouldn't happen
                Message::Binary(_) => unimplemented!(),
                // ignore
                Message::Pong(_) => {}
                // ignore, will be handled by axum
                Message::Ping(_) => {}
            }
        }
        println!("recv_task has nothing more to receive, exiting");
        Ok(())
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        bg_light_join_result = &mut bg_light_task => {
            send_task.abort();
            recv_task.abort();
            dbg!(bg_light_join_result)
                .context(format!("Panic in a bg light level task of {who}"))?
                .context(format!("Error in a bg light level task of {who}"))?;
        },
        send_join_result = &mut send_task => {
            bg_light_task.abort();
            recv_task.abort();
            dbg!(send_join_result)
                .context(format!("Panic in a websocket send task of {who}"))?
                .context(format!("Error in a websocket send task of {who}"))?;
        },
        recv_join_result = &mut recv_task => {
            bg_light_task.abort();
            send_task.abort();
            // recv_join_result is swallowed here somehow?
            dbg!(recv_join_result)
                .context(format!("Panic in a websocket recv task of {who}"))?
                .context(format!("Error in a websocket recv task of {who}"))?;
        }
    }

    println!("Closing {who} websocket");
    Ok(())
}

async fn handle_message(
    msg: api::ClientToServer,
    device: &TokioMutex<Device>,
    to_client_sender: &mpsc::Sender<api::ServerToClient>,
    bg_streaming_enabled_sender: &watch::Sender<bool>,
) -> anyhow::Result<()> {
    use api::ClientToServer as CTS;
    match msg {
        CTS::Status => {
            let device_status = {
                let mut device = device.lock().await;
                device.get_status().await?
            };
            to_client_sender
                .send(api::ServerToClient::Status {
                    version: api::Version {
                        hardware: device_status.version.hardware,
                        firmware: device_status.version.firmware,
                    },
                    max_light_level: device_status.max_light_level,
                })
                .await?;
        }
        CTS::StartMonitoring => {
            bg_streaming_enabled_sender.send(true)?;
        }
        CTS::StopMonitoring => {
            bg_streaming_enabled_sender.send(false)?;
        }
        CTS::SendHidReport { hid_report } => {
            let mut device = device.lock().await;
            device.send_hid_report(&hid_report).await?;
        }
        CTS::Measure {
            before,
            duration_ms,
            start,
            followup,
            after,
        } => {
            let (max_light_level, measurements) = {
                let mut device = device.lock().await;

                let status = device.get_status().await?;

                for report in before {
                    device.send_hid_report(&report).await?;
                }

                let measurements = device
                    .measure(
                        duration_ms,
                        &start,
                        followup.map(|f| (f.after_ms, f.hid_report)),
                    )
                    .await?;

                for report in after {
                    device.send_hid_report(&report).await?;
                }

                (status.max_light_level, measurements)
            };

            let processed = ProcessedMeasurements::new(&measurements)?;

            to_client_sender
                .send(api::ServerToClient::Measurement {
                    max_light_level,
                    light_levels: processed.light_levels,
                    followup_hid_us: processed.followup_hid_us,
                    change_us: processed.change_us,
                })
                .await?;
        }
    }
    Ok(())
}

// realigns light_levels so that the start HID event = 0
struct ProcessedMeasurements {
    /// microsecond, light level
    light_levels: Vec<(u32, u32)>,
    followup_hid_us: Option<u32>,
    change_us: Option<u32>,
}

impl ProcessedMeasurements {
    fn new(measurements: &[Measurement]) -> anyhow::Result<Self> {
        use late_mate_shared::MeasurementEvent as ME;

        let (first_hid_idx, first_hid_time) = measurements
            .iter()
            .enumerate()
            .find_map(|(idx, m)| match m.event {
                ME::LightLevel(_) => None,
                ME::HidReport(_) => Some((idx, m.microsecond)),
            })
            .context("No HID report in returned measurements")?;

        let followup_hid_us =
            measurements
                .iter()
                .skip(first_hid_idx + 1)
                .find_map(|m| match m.event {
                    ME::LightLevel(_) => None,
                    ME::HidReport(_) => Some(m.microsecond - first_hid_time),
                });

        let light_levels = measurements
            .iter()
            .skip(first_hid_idx + 1)
            .filter_map(|m| match m.event {
                ME::LightLevel(l) => Some((m.microsecond - first_hid_time, l)),
                ME::HidReport(_) => None,
            })
            .collect::<Vec<_>>();

        // todo: For some reason (channel shenanigans?), sometimes I get a "tail" (?)
        //       in the beginning of the values (a tuple with an unreasonably high time).
        //       This just filters out the tail
        let mut filtered_light_levels = Vec::with_capacity(light_levels.len());
        let mut last_time = light_levels.last().unwrap().0;
        let initial_light_levels_len = light_levels.len();
        for entry @ (time, _) in light_levels.into_iter().rev() {
            if time <= last_time {
                filtered_light_levels.push(entry)
            }
            last_time = time;
        }
        filtered_light_levels.reverse();

        let filtered_out = initial_light_levels_len - filtered_light_levels.len();
        if filtered_out > 0 {
            dbg!(filtered_out);
        }

        let change_us = find_changepoint(&filtered_light_levels);

        Ok(ProcessedMeasurements {
            light_levels: filtered_light_levels,
            followup_hid_us,
            change_us,
        })
    }
}

fn find_changepoint(light_levels: &[(u32, u32)]) -> Option<u32> {
    // it's unlikely there's any meaningful change in the first 7ms,
    // so I use it to infer the range of noise
    let noise_window = 7_000;
    // require at least 2 noise ranges between start and end to detect change
    let change_detect_gap_multiplier = 2;
    // but for the actual moment of change, use just one noise range
    let change_gap_multiplier = 1;

    let (start_min, start_max) = light_levels
        .iter()
        .copied()
        .take_while(|(us, _)| *us < noise_window)
        .fold((u32::MAX, 0u32), |(min, max), (_, light_level)| {
            (light_level.min(min), light_level.max(max))
        });

    // dbg!(start_min, start_max);

    let last_time = light_levels
        .last()
        .expect("light_levels shouldn't be empty at this point")
        .0;

    let (end_min, end_max) = light_levels
        .iter()
        .rev()
        .copied()
        .take_while(|(us, _)| *us > (last_time - noise_window))
        .fold((u32::MAX, 0u32), |(min, max), (_, light_level)| {
            (light_level.min(min), light_level.max(max))
        });

    // dbg!(end_min, end_max);

    let change_detect_gap = (start_max - start_min) * change_detect_gap_multiplier;

    if !(end_min > (start_max + change_detect_gap) || start_min > (end_max + change_detect_gap)) {
        // println!("no change detected");
        return None;
    }

    let change_gap = (start_max - start_min) * change_gap_multiplier;
    let change_point = if end_min > start_max {
        // raising signal
        let threshold = start_max + change_gap;
        light_levels.iter().copied().find_map(|(us, light_level)| {
            if light_level > threshold {
                Some(us)
            } else {
                None
            }
        })
    } else {
        // dropping signal, non-changing signal is already discarded above
        // there must be enough between the ends for this sum to be always positive
        assert!(
            start_min > change_gap,
            "expected started_min ({start_min}) > change_gap ({change_gap})"
        );
        let threshold = start_min - change_gap;
        light_levels.iter().copied().find_map(|(us, light_level)| {
            if light_level < threshold {
                Some(us)
            } else {
                None
            }
        })
    };
    // dbg!(change_point);

    Some(change_point.expect("the signal must cross the threshold given the above"))
}
