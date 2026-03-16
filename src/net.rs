use std::sync::mpsc;
use std::thread;

use crate::protocol::{ClientMsg, ServerMsg, DEFAULT_PORT};

pub struct NetClient {
    pub tx: mpsc::Sender<ClientMsg>,
    _handle: thread::JoinHandle<()>,
}

impl NetClient {
    /// Connect to the server. Returns the client handle and a receiver for server messages.
    /// The network runs on a background thread with its own tokio runtime.
    pub fn connect(addr: &str) -> Result<(Self, mpsc::Receiver<ServerMsg>), String> {
        let url = format!("ws://{addr}:{DEFAULT_PORT}");
        let (server_tx, server_rx) = mpsc::channel::<ServerMsg>();
        let (client_tx, client_rx) = mpsc::channel::<ClientMsg>();

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                use futures_util::{SinkExt, StreamExt};
                use tokio_tungstenite::tungstenite::Message;

                let ws = match tokio_tungstenite::connect_async(&url).await {
                    Ok((ws, _)) => ws,
                    Err(e) => {
                        let _ = server_tx.send(ServerMsg::Error {
                            msg: format!("Connection failed: {e}"),
                        });
                        return;
                    }
                };

                let (mut ws_tx, mut ws_rx) = ws.split();

                // Bridge: poll both the client_rx (std mpsc) and the ws_rx (async)
                // using a tokio interval to check the std channel
                let (outgoing_tx, mut outgoing_rx) =
                    tokio::sync::mpsc::unbounded_channel::<String>();

                // Writer: sends queued messages to WebSocket
                tokio::spawn(async move {
                    while let Some(json) = outgoing_rx.recv().await {
                        if ws_tx.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                });

                // Poller: drains the std mpsc channel into the async outgoing channel
                let outgoing_tx2 = outgoing_tx.clone();
                let poll_handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(20));
                    loop {
                        interval.tick().await;
                        loop {
                            match client_rx.try_recv() {
                                Ok(msg) => {
                                    let json = serde_json::to_string(&msg).unwrap();
                                    if outgoing_tx2.send(json).is_err() {
                                        return;
                                    }
                                }
                                Err(mpsc::TryRecvError::Empty) => break,
                                Err(mpsc::TryRecvError::Disconnected) => return,
                            }
                        }
                    }
                });

                // Reader: WebSocket → server_tx
                while let Some(Ok(msg)) = ws_rx.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
                            if server_tx.send(server_msg).is_err() {
                                break;
                            }
                        }
                    }
                }

                poll_handle.abort();
            });
        });

        Ok((NetClient { tx: client_tx, _handle: handle }, server_rx))
    }

    pub fn send(&self, msg: ClientMsg) {
        let _ = self.tx.send(msg);
    }
}
