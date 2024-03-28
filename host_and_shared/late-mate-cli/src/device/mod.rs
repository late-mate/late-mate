use crate::device::rxtx::{rx_loop, tx_loop, CrcCobsCodec};
use crate::device::serial::find_serial_port;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use late_mate_comms::{DeviceToHost, HostToDevice, Status};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

mod rxtx;
mod serial;

pub struct Device {
    tx_sender: mpsc::Sender<HostToDevice>,
    rx_receiver: broadcast::Receiver<DeviceToHost>,
    max_light_level: u32,
}

impl Device {
    pub async fn init() -> anyhow::Result<Self> {
        let serial_port_info = find_serial_port()?;
        let serial_stream = tokio_serial::new(serial_port_info.port_name, 115200)
            .open_native_async()
            .context("Serial port opening failure")?;

        let serial_framed = Framed::new(serial_stream, CrcCobsCodec::new());
        let (serial_framed_tx, serial_framed_rx) = serial_framed.split();

        let (tx_sender, tx_receiver) = mpsc::channel(4);
        let (rx_sender, rx_receiver) = broadcast::channel(1);

        // todo: handle those handles
        let _tx_loop_handle = tokio::spawn(tx_loop(serial_framed_tx, tx_receiver));
        let _rx_loop_handle = tokio::spawn(rx_loop(serial_framed_rx, rx_sender.clone()));

        let mut tmp_self = Self {
            tx_sender,
            rx_receiver,
            max_light_level: 0,
        };
        let device_status = tmp_self.get_status().await?;
        tmp_self.max_light_level = device_status.max_light_level;

        Ok(tmp_self)
    }

    pub async fn get_status(&mut self) -> anyhow::Result<Status> {
        self.one_off(
            HostToDevice::GetStatus,
            Some(|msg| match msg {
                DeviceToHost::Status(s) => Some(s),
                _ => None,
            }),
        )
        .await
    }

    async fn one_off<T>(
        &mut self,
        command: HostToDevice,
        resp_mapper: Option<impl Fn(DeviceToHost) -> Option<T>>,
    ) -> anyhow::Result<T> {
        let req_future = async {
            self.tx_sender
                .send(command)
                .await
                .context("Device TX channel was unexpectedly closed")
        };
        let resp_future = async {
            loop {
                match self.rx_receiver.recv().await {
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
