use crate::device::usb::ALIGNED_BUFFER_SIZE;
use crate::device::DeviceError;
use anyhow::Context;
use late_mate_shared::comms;
use late_mate_shared::comms::host_to_device;
use nusb::transfer;
use std::mem;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

async fn tx_loop(
    mut out_queue: transfer::Queue<Vec<u8>>,
    mut receiver: mpsc::Receiver<host_to_device::Envelope>,
) -> anyhow::Result<()> {
    let n_transfers = 8;

    let mut next_buf = vec![0; ALIGNED_BUFFER_SIZE];

    loop {
        while out_queue.pending() < n_transfers {
            let mut buf = mem::replace(&mut next_buf, vec![0; ALIGNED_BUFFER_SIZE]);

            let envelope = match receiver.recv().await {
                Some(m) => m,
                None => {
                    // all senders are dropped, just exit
                    // todo: maybe log this?
                    return Ok(());
                }
            };
            let used_len = comms::encode(&envelope, buf.as_mut_slice());
            buf.truncate(used_len);

            out_queue.submit(buf);
        }

        let resp_buf = out_queue
            .next_complete()
            .await
            .into_result()
            .context("USB error while sending data")?;
        next_buf = resp_buf.reuse();
    }
}

#[derive(Debug, Clone)]
pub struct TxHandle {
    sender: mpsc::Sender<host_to_device::Envelope>,
}

impl TxHandle {
    pub async fn send(&self, envelope: host_to_device::Envelope) -> Result<(), DeviceError> {
        self.sender
            .send(envelope)
            .await
            .map_err(|_| DeviceError::Disconnected)
    }
}

pub fn start(
    agent_set: &mut JoinSet<anyhow::Result<()>>,
    out_queue: transfer::Queue<Vec<u8>>,
) -> TxHandle {
    let (sender, receiver) = mpsc::channel::<host_to_device::Envelope>(16);

    agent_set.spawn(tx_loop(out_queue, receiver));

    TxHandle { sender }
}
