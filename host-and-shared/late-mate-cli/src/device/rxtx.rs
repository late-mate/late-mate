use anyhow::{anyhow, Context};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use late_mate_shared::{
    CrcCobsAccumulator, DeviceToHost, FeedResult, HostToDevice, MAX_BUFFER_SIZE,
};
use tokio::sync::{broadcast, mpsc};
use tokio_serial::SerialStream;
use tokio_util::bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed};

pub struct CrcCobsCodec {
    acc: CrcCobsAccumulator,
}

impl CrcCobsCodec {
    pub fn new() -> Self {
        Self {
            acc: CrcCobsAccumulator::new(),
        }
    }
}

impl Decoder for CrcCobsCodec {
    type Item = DeviceToHost;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.acc.feed::<DeviceToHost>(src) {
            FeedResult::Consumed => {
                // todo: can this be done cleaner?
                src.advance(src.len());
                Ok(None)
            }
            FeedResult::OverFull { .. } => Err(anyhow!("USB buffer overflow")),
            FeedResult::Error { error: e, .. } => {
                Err(anyhow!("Serial packet decoding failure ({e:?})"))
            }
            FeedResult::Success { data, remaining } => {
                src.advance(src.len() - remaining.len());
                Ok(Some(data))
            }
        }
    }
}

impl Encoder<HostToDevice> for CrcCobsCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: HostToDevice, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut msg_buf = [0u8; MAX_BUFFER_SIZE];
        let msg_len = late_mate_shared::encode(&item, &mut msg_buf);

        dst.extend_from_slice(&msg_buf[..msg_len]);

        Ok(())
    }
}

// RX (device->host)
pub async fn rx_loop(
    mut serial_rx: SplitStream<Framed<SerialStream, CrcCobsCodec>>,
    rx_sender: broadcast::Sender<DeviceToHost>,
) -> anyhow::Result<()> {
    loop {
        let msg = serial_rx
            .next()
            .await
            .expect("Serial stream seems to be closed")
            .context("Error reading device message")?;
        //dbg!(msg);
        rx_sender
            .send(msg)
            .context("Error broadcasting device message")?;
    }
}

// TX (host->device)
pub async fn tx_loop(
    mut serial_tx: SplitSink<Framed<SerialStream, CrcCobsCodec>, HostToDevice>,
    mut device_tx_receiver: mpsc::Receiver<HostToDevice>,
) -> anyhow::Result<()> {
    // .recv() will return None when all tx senders are dropped, ie there can be no more
    // tx messages, so it's fine to just exit when that happens
    while let Some(msg) = device_tx_receiver.recv().await {
        //dbg!(msg);
        serial_tx.send(msg).await?;
    }
    Ok(())
}
