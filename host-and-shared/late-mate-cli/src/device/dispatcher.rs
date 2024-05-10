use crate::device::DeviceError;
use anyhow::anyhow;
use late_mate_shared::comms::device_to_host::DeviceToHost;
use late_mate_shared::comms::host_to_device::{HostToDevice, RequestId};
use late_mate_shared::comms::{device_to_host, host_to_device};
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::task::JoinSet;
use tokio::time::sleep;

#[derive(Debug, Default)]
pub struct Dispatcher {
    next_request_id: RequestId,
    pending: BTreeMap<RequestId, mpsc::Sender<Result<Option<DeviceToHost>, DeviceError>>>,
}

impl Dispatcher {
    fn new_request_id(&mut self) -> RequestId {
        let next_request_id = self.next_request_id.wrapping_add(1);
        mem::replace(&mut self.next_request_id, next_request_id)
    }

    pub fn register_request(
        &mut self,
        request: HostToDevice,
    ) -> (
        mpsc::Receiver<Result<Option<DeviceToHost>, DeviceError>>,
        host_to_device::Envelope,
    ) {
        let request_id = self.new_request_id();
        let envelope = host_to_device::Envelope {
            request_id,
            request,
        };

        let (sender, receiver) = mpsc::channel(32);
        let is_new = self.pending.insert(request_id, sender.clone()).is_none();
        assert!(is_new, "There should be no duplicate requests");

        (receiver, envelope)
    }
}

async fn dispatcher_loop(
    dispatcher: Arc<TokioMutex<Dispatcher>>,
    mut rx_receiver: mpsc::Receiver<device_to_host::Envelope>,
) -> anyhow::Result<()> {
    loop {
        match rx_receiver.recv().await {
            Some(envelope) => {
                let request_id = envelope.request_id;
                let pending_sender = dispatcher.lock().await.pending.get(&request_id).cloned();

                if let Some(pending_sender) = pending_sender {
                    let response = envelope.response.map_err(|_| DeviceError::OnDeviceError);
                    match pending_sender.send(response).await {
                        Ok(_) => continue,
                        Err(_) => {
                            // The receiver is dropped, no point trying to send in the future.
                            // It can't reappear either, so racing the lock above is no issue
                            dispatcher.lock().await.pending.remove(&request_id);
                        }
                    }
                }
            }
            None => {
                // USB must be closed, let the pending requests know & exit
                // todo: maybe log this?
                let pending = mem::take(&mut dispatcher.lock().await.pending);
                for sender in pending.values() {
                    let _ = sender.send(Err(DeviceError::Disconnected)).await;
                }
                return Ok(());
            }
        }
    }
}

async fn reaper_loop(dispatcher: Arc<TokioMutex<Dispatcher>>) -> anyhow::Result<()> {
    loop {
        dispatcher
            .lock()
            .await
            .pending
            .retain(|_, sender| !sender.is_closed());

        sleep(Duration::from_secs(5)).await;
    }
}

pub fn start(
    join_set: &mut JoinSet<anyhow::Result<()>>,
    rx_receiver: mpsc::Receiver<device_to_host::Envelope>,
) -> Arc<TokioMutex<Dispatcher>> {
    let dispatcher = Arc::new(TokioMutex::new(Dispatcher::default()));

    join_set.spawn(dispatcher_loop(dispatcher.clone(), rx_receiver));
    join_set.spawn(reaper_loop(dispatcher.clone()));

    dispatcher
}
