//! Runtime implementation

use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// Handle to runtime
#[derive(Debug)]
pub struct Runtime {
    /// Sender to backing runtime task
    evloop_tx: mpsc::Sender<Message>,
}

impl Runtime {
    /// Create a new runtime handle
    pub fn new() -> Self {
        let (evloop_tx, mut evloop_rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut evloop = EventLoop::new();
            while let Some(msg) = evloop_rx.recv().await {
                evloop.handle_msg(msg);
            }
        });
        Self { evloop_tx }
    }

    /// Return number of connected clients in runtime
    pub async fn number_of_clients(&self) -> Result<usize> {
        let (req_tx, req_rx) = oneshot::channel();
        self.evloop_tx
            .send(Message::GetNumberOfClients(req_tx))
            .await?;
        Ok(req_rx.await?)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Messages passed between [Runtime] and [RuntimeTask] event loop
pub enum Message {
    GetNumberOfClients(oneshot::Sender<usize>),
}

/// The main event loop backing runtime
#[derive(Debug)]
struct EventLoop {
    clients: Vec<JoinHandle<()>>,
}

impl EventLoop {
    fn new() -> Self {
        Self { clients: vec![] }
    }

    /// Handle a message in event loop
    fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::GetNumberOfClients(resp_tx) => {
                let _ = resp_tx.send(self.clients.len());
            }
        }
    }
}

/// Errors from [Runtime]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to send message to event loop - {0}")]
    FailedToSendToEventLoop(#[from] SendError<Message>),

    #[error("Failed to receive response from event loop - {0}")]
    FailedToReceiveResponse(#[from] tokio::sync::oneshot::error::RecvError),
}

/// Result type for [Runtime]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn runtime_init() {
        let runtime = Runtime::new();
        assert_eq!(runtime.number_of_clients().await.unwrap(), 0);
    }
}
