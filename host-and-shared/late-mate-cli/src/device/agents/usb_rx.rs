use crate::device::usb::ALIGNED_BUFFER_SIZE;
use anyhow::Context;
use late_mate_shared::comms;
use late_mate_shared::comms::{device_to_host, CrcCobsAccumulator, FeedResult};
use nusb::transfer;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Debug, thiserror::Error)]
enum ProcessingError {
    #[error("CRC/COBS buffer is overfull")]
    BufferOverfull,
    #[error("Postcard error: {0:?}")]
    PostcardError(comms::PostcardError),
    #[error("Channel receiver was dropped")]
    ReceiverGone,
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
                    .map_err(|_| ProcessingError::ReceiverGone)?;
                remaining
            }
        }
    }

    Ok(())
}

async fn rx_loop(
    mut in_queue: transfer::Queue<transfer::RequestBuffer>,
    sender: mpsc::Sender<device_to_host::Envelope>,
) -> anyhow::Result<()> {
    // this sets up a number of buffers that the kernel will later fill in
    let n_transfers = 8;
    while in_queue.pending() < n_transfers {
        in_queue.submit(transfer::RequestBuffer::new(ALIGNED_BUFFER_SIZE));
    }

    let mut cobs_acc = CrcCobsAccumulator::new();

    loop {
        let completion = in_queue.next_complete().await;
        completion
            .status
            .context("USB error while receiving data")?;
        match process_packet(&sender, &mut cobs_acc, completion.data.as_slice()).await {
            Ok(_) => (),
            Err(ProcessingError::ReceiverGone) => {
                // no point continuing, just exit
                // todo: maybe log this?
                return Ok(());
            }
            other @ Err(_) => return other.context("Error while processing a USB packet"),
        }

        in_queue.submit(transfer::RequestBuffer::reuse(
            completion.data,
            ALIGNED_BUFFER_SIZE,
        ));
    }
}

#[derive(Debug)]
pub struct RxHandle {
    receiver: mpsc::Receiver<device_to_host::Envelope>,
}

impl RxHandle {
    pub async fn recv(&mut self) -> Option<device_to_host::Envelope> {
        self.receiver.recv().await
    }
}

pub fn start(
    agent_set: &mut JoinSet<anyhow::Result<()>>,
    in_queue: transfer::Queue<transfer::RequestBuffer>,
) -> RxHandle {
    let (sender, receiver) = mpsc::channel(16);

    agent_set.spawn(rx_loop(in_queue, sender));

    RxHandle { receiver }
}
