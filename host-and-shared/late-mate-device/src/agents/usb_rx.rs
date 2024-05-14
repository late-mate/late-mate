use crate::usb::ALIGNED_BUFFER_SIZE;
use late_mate_shared::comms;
use late_mate_shared::comms::{device_to_host, CrcCobsAccumulator, FeedResult};
use nusb::transfer;
use nusb::transfer::TransferError;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Debug, thiserror::Error)]
enum ProcessingError {
    #[error("CRC/COBS buffer is overfull")]
    BufferOverfull,
    #[error("Postcard error: {0:?}")]
    PostcardError(comms::PostcardError),
    #[error("Downstream channel is closed")]
    ChannelClosed,
}

async fn process_packet(
    sender: &mpsc::Sender<device_to_host::Envelope>,
    cobs_acc: &mut CrcCobsAccumulator,
    mut packet: &[u8],
) -> Result<(), ProcessingError> {
    'cobs: while !packet.is_empty() {
        packet = match cobs_acc.feed::<device_to_host::Envelope>(packet) {
            FeedResult::Consumed => break 'cobs,
            FeedResult::OverFull { .. } => {
                return Err(ProcessingError::BufferOverfull);
            }
            FeedResult::Error { error: e, .. } => {
                return Err(ProcessingError::PostcardError(e));
            }
            FeedResult::Success { data, remaining } => {
                sender
                    .send(data)
                    .await
                    .map_err(|_| ProcessingError::ChannelClosed)?;
                remaining
            }
        }
    }

    Ok(())
}

async fn usb_rx_loop(
    mut in_queue: transfer::Queue<transfer::RequestBuffer>,
    sender: mpsc::Sender<device_to_host::Envelope>,
) {
    // this sets up a number of buffers that the kernel will later fill in
    let n_transfers = 8;
    while in_queue.pending() < n_transfers {
        in_queue.submit(transfer::RequestBuffer::new(ALIGNED_BUFFER_SIZE));
    }

    let mut cobs_acc = CrcCobsAccumulator::new();

    loop {
        let completion = in_queue.next_complete().await;

        // I can't associate the error with a particular request anyway, so the best I can do
        // is to log and swallow errors + rely on timeouts,
        // The only exception is the Disconnected error, there is no point going on
        match completion.status {
            Err(TransferError::Disconnected) => {
                tracing::error!("USB device disconnected, USB RX loop exiting");
                break;
            }
            Err(e) => {
                tracing::error!("USB RX error: {e}");
            }
            Ok(_) => {
                match process_packet(&sender, &mut cobs_acc, completion.data.as_slice()).await {
                    Ok(_) => (),
                    Err(ProcessingError::ChannelClosed) => {
                        tracing::info!("Envelope receiver is dropped, USB RX loop exiting");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("USB packet deserialisation error: {e}");
                    }
                }
            }
        }

        in_queue.submit(transfer::RequestBuffer::reuse(
            completion.data,
            ALIGNED_BUFFER_SIZE,
        ));
    }
}

#[derive(Debug)]
pub struct UsbRxHandle {
    receiver: mpsc::Receiver<device_to_host::Envelope>,
}

impl UsbRxHandle {
    pub async fn recv(&mut self) -> Option<device_to_host::Envelope> {
        self.receiver.recv().await
    }
}

pub fn start(
    agent_set: &mut JoinSet<()>,
    in_queue: transfer::Queue<transfer::RequestBuffer>,
) -> UsbRxHandle {
    let (sender, receiver) = mpsc::channel(16);

    agent_set.spawn(usb_rx_loop(in_queue, sender));

    UsbRxHandle { receiver }
}
