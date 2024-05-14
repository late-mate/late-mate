use crate::usb::ALIGNED_BUFFER_SIZE;
use crate::Error;
use late_mate_shared::comms;
use late_mate_shared::comms::host_to_device;
use nusb::transfer;
use nusb::transfer::TransferError;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

async fn usb_tx_loop(
    mut out_queue: transfer::Queue<Vec<u8>>,
    mut receiver: mpsc::Receiver<(host_to_device::Envelope, oneshot::Sender<Error>)>,
) {
    let mut buf = vec![0; ALIGNED_BUFFER_SIZE];

    // Just 1 pending request in the queue to make it easy to associate errors and submissions.
    // I'm not sure if I even can figure out which submission caused an error in `next_complete`?
    loop {
        let (envelope, reply_error_to) = match receiver.recv().await {
            Some(m) => m,
            None => {
                tracing::info!("All senders are dropped, USB TX loop exiting");
                break;
            }
        };
        dbg!(&envelope);
        dbg!(buf.len());
        dbg!(buf.capacity());
        let used_len = comms::encode(&envelope, buf.as_mut_slice());
        buf.truncate(used_len);

        out_queue.submit(buf);

        let completion = out_queue.next_complete().await;

        if let Err(e) = completion.status {
            // if the receiver was dropped, the error doesn't matter anyway
            _ = reply_error_to.send(Error::UsbTransferError("sending data", e));

            if matches!(e, TransferError::Disconnected) {
                tracing::error!("USB device disconnected, USB TX loop exiting");
                break;
            }
        }

        buf = completion.data.reuse();
        // I get back the same vector truncated to 0, inflate it back
        buf.resize(ALIGNED_BUFFER_SIZE, 0);
    }
}

#[derive(Debug, Clone)]
pub struct UsbTxHandle {
    sender: mpsc::Sender<(host_to_device::Envelope, oneshot::Sender<Error>)>,
}

impl UsbTxHandle {
    pub async fn send(&self, envelope: host_to_device::Envelope) -> Result<(), Error> {
        let (error_sender, error_receiver) = oneshot::channel();

        self.sender
            .send((envelope, error_sender))
            .await
            .map_err(|_| Error::Disconnected)?;

        match error_receiver.await {
            Ok(e) => Err(e),
            Err(_) => Ok(()),
        }
    }
}

pub fn start(agent_set: &mut JoinSet<()>, out_queue: transfer::Queue<Vec<u8>>) -> UsbTxHandle {
    let (sender, receiver) = mpsc::channel(4);

    agent_set.spawn(usb_tx_loop(out_queue, receiver));

    UsbTxHandle { sender }
}
