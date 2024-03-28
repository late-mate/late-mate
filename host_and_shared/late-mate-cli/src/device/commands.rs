use anyhow::{anyhow, Context, Error};
use late_mate_comms::{DeviceToHost, HostToDevice, Status};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

async fn one_off<T>(
    device_tx: mpsc::Sender<HostToDevice>,
    mut device_rx: broadcast::Receiver<DeviceToHost>,
    command: HostToDevice,
    resp_mapper: Option<impl Fn(DeviceToHost) -> Option<T>>,
) -> anyhow::Result<T> {
    let req_future = async move {
        device_tx
            .send(command)
            .await
            .context("Device TX channel was unexpectedly closed")
    };
    let resp_future = async move {
        loop {
            match device_rx.recv().await {
                Ok(msg) => {
                    if let Some(mapper) = &resp_mapper {
                        match mapper(msg) {
                            Some(result) => return Ok(result),
                            None => continue,
                        }
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => {
                    return Err(anyhow!("Device RX channel was unexpectedly closed"))
                }
            }
        }
    };
    let timely_resp_future = async move {
        match timeout(Duration::from_secs(3), resp_future).await {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("Timeout while waiting for response to {command:?}")),
        }
    };

    match tokio::try_join!(req_future, timely_resp_future) {
        Ok((_, status)) => Ok(status),
        // have to rewrap to change the type ('Ok' branches are different)
        Err(e) => Err(e.into()),
    }
}

pub async fn get_status(
    device_tx: mpsc::Sender<HostToDevice>,
    device_rx: broadcast::Receiver<DeviceToHost>,
) -> anyhow::Result<Status> {
    one_off(
        device_tx,
        device_rx,
        HostToDevice::GetStatus,
        Some(|msg| match msg {
            DeviceToHost::Status(s) => Some(s),
            _ => None,
        }),
    )
    .await
}
