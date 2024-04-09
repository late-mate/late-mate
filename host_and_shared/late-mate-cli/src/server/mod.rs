pub mod api;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

use anyhow::Context;
use std::future::Future;
use std::sync::Arc;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::extract::connect_info::ConnectInfo;

use crate::device::Device;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::task::JoinHandle;

#[derive(Clone)]
struct ServerState {
    device: Arc<TokioMutex<Device>>,
}

pub async fn run(device: Device) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "late-mate-cli=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    // build our application with some routes
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))
        // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(ServerState {
            device: Arc::new(TokioMutex::new(device)),
        }));

    let address = "127.0.0.1:1838";
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .context(format!("Couldn't bind on {address}"))?;
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

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (to_client_sender, mut to_client_receiver) = mpsc::channel::<api::ServerToClient>(4);

    let mut send_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(msg) = to_client_receiver.recv().await {
            ws_sender
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await?;
        }
        Ok(())
    });

    let mut recv_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(txt) => {
                    let from_client: api::ClientToServer = serde_json::from_str(&txt)?;
                    handle_message(from_client, device.as_ref(), &to_client_sender).await?;
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
        Ok(())
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        send_join_result = &mut send_task => {
            recv_task.abort();
            send_join_result
                .context(format!("Panic in a websocket send task of {who}"))?
                .context(format!("Error in a websocket send task of {who}"))?;
        },
        recv_join_result = &mut recv_task => {
            send_task.abort();
            recv_join_result
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
        CTS::StartMonitoring => {}
        CTS::StopMonitoring => {}
        CTS::SendHidReport { .. } => {}
        CTS::Measure { .. } => {}
    }
    Ok(())
}
