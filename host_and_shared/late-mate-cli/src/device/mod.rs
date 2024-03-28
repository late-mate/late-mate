use anyhow::{anyhow, Context};
use late_mate_comms::{DeviceToHost, HostToDevice};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

pub mod commands;
pub mod rxtx;
pub mod serial;

struct Device {
    device_tx: mpsc::Sender<HostToDevice>,
    device_rx: broadcast::Receiver<DeviceToHost>,
}

impl Device {
    async fn one_off<T>(
        &mut self,
        command: HostToDevice,
        resp_mapper: Option<impl Fn(DeviceToHost) -> Option<T>>,
    ) -> anyhow::Result<T> {
        let req_future = async {
            self.device_tx
                .send(command)
                .await
                .context("Device TX channel was unexpectedly closed")
        };
        let resp_future = async {
            loop {
                match self.device_rx.recv().await {
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
            Err(e) => Err(e),
        }
    }
}
