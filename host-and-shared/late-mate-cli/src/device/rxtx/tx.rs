use crate::device::rxtx::ALIGNED_BUFFER_SIZE;
use anyhow::Context;
use late_mate_shared::comms;
use late_mate_shared::comms::{host_to_device, usb_interface};
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

pub fn start(
    join_set: &mut JoinSet<anyhow::Result<()>>,
    out_queue: transfer::Queue<Vec<u8>>,
) -> mpsc::Sender<host_to_device::Envelope> {
    let (sender, receiver) = mpsc::channel::<host_to_device::Envelope>(16);

    join_set.spawn(tx_loop(out_queue, receiver));

    sender
}
