use crate::agents::usb_rx::UsbRxHandle;
use crate::{Error, ResponseResult};
use late_mate_shared::comms::{device_to_host, host_to_device};
use std::collections::BTreeMap;
use std::mem;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time;
use tokio::time::MissedTickBehavior;

#[derive(Debug, Default)]
struct State {
    next_request_id: host_to_device::RequestId,
    pending: BTreeMap<host_to_device::RequestId, mpsc::Sender<ResponseResult>>,
}

enum Command {
    RegisterRequest {
        request: host_to_device::Message,
        reply_to: oneshot::Sender<(mpsc::Receiver<ResponseResult>, host_to_device::Envelope)>,
    },
}

impl State {
    fn new_request_id(&mut self) -> host_to_device::RequestId {
        let next_request_id = self.next_request_id.wrapping_add(1);
        mem::replace(&mut self.next_request_id, next_request_id)
    }

    async fn handle_usb_rx(&mut self, envelope: device_to_host::Envelope) {
        let request_id = envelope.request_id;
        let pending_sender = self.pending.get(&request_id).cloned();

        if let Some(pending_sender) = pending_sender {
            let response = envelope.response.map_err(|_| Error::OnDeviceError);
            if pending_sender.send(response).await.is_err() {
                // The receiver is dropped, no point trying to send in the future.
                // It can't reappear either, so racing the lock above is no issue
                self.pending.remove(&request_id);
            }
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::RegisterRequest { request, reply_to } => {
                let request_id = self.new_request_id();
                let envelope = host_to_device::Envelope {
                    request_id,
                    request,
                };

                let (sender, receiver) = mpsc::channel(1);
                let is_new = self.pending.insert(request_id, sender.clone()).is_none();
                assert!(is_new, "There should be no duplicate requests");

                if reply_to.send((receiver, envelope)).is_err() {
                    // Whoever requested this has died, no point having it around
                    self.pending.remove(&request_id);
                }
            }
        }
    }

    fn reap(&mut self) {
        self.pending.retain(|_, sender| !sender.is_closed());
    }
}

async fn dispatcher_loop(
    mut state: State,
    mut rx: UsbRxHandle,
    mut command_receiver: mpsc::Receiver<Command>,
) {
    let mut reaping_interval = time::interval(Duration::from_secs(5));
    reaping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            usb_rx = rx.recv() => match usb_rx {
                Some(envelope) => state.handle_usb_rx(envelope).await,
                None => {
                    tracing::info!("USB RX loop is dropped, dispatcher exiting");
                    // Let the pending requests know we're down
                    let pending = mem::take(&mut state.pending);
                    for sender in pending.values() {
                        let _ = sender.send(Err(Error::Disconnected)).await;
                    }
                    break
                },
            },
            command = command_receiver.recv() => match command {
                Some(command) => state.handle_command(command),
                None => {
                    tracing::info!("Command channel is dropped, dispatcher exiting");
                    break
                },
            },
            _ = reaping_interval.tick() => state.reap()
        }
    }
}

#[derive(Debug, Clone)]
pub struct DispatcherHandle {
    sender: mpsc::Sender<Command>,
}

impl DispatcherHandle {
    pub async fn register_request(
        &self,
        request: host_to_device::Message,
    ) -> (mpsc::Receiver<ResponseResult>, host_to_device::Envelope) {
        let (reply_to, reply_to_receiver) = oneshot::channel();
        let command = Command::RegisterRequest { request, reply_to };

        // if Dispatcher is dead, we'll fail below regardless
        let _ = self.sender.send(command).await;

        reply_to_receiver.await.expect("Dispatcher must be alive")
    }
}

pub fn start(agent_set: &mut JoinSet<()>, rx: UsbRxHandle) -> DispatcherHandle {
    let (sender, command_receiver) = mpsc::channel(1);
    let state = State::default();

    agent_set.spawn(dispatcher_loop(state, rx, command_receiver));

    DispatcherHandle { sender }
}
