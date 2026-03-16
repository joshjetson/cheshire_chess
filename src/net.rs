use std::sync::mpsc;
use std::thread;

use crate::protocol::{ClientMsg, ServerMsg};

/// Central game server URL — all players connect here.
const GAME_SERVER_URL: &str = "wss://chess.virtualraremedia.com/ws";

pub struct NetClient {
    pub tx: mpsc::Sender<ClientMsg>,
    _handle: thread::JoinHandle<()>,
}

impl NetClient {
    /// Connect to the central game server via WSS through Caddy.
    pub fn connect() -> Result<(Self, mpsc::Receiver<ServerMsg>), String> {
        Self::connect_to(GAME_SERVER_URL)
    }

    /// Connect to a specific WebSocket URL (for LAN/local dev).
    pub fn connect_to(url: &str) -> Result<(Self, mpsc::Receiver<ServerMsg>), String> {
        let url = url.to_string();
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

                let connector = if url.starts_with("wss://") {
                    // TLS connector using rustls
                    let root_store = rustls::RootCertStore::from_iter(
                        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
                    );
                    let config = rustls::ClientConfig::builder()
                        .with_root_certificates(root_store)
                        .with_no_client_auth();
                    Some(tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(config)))
                } else {
                    None
                };

                let ws_result = if let Some(conn) = connector {
                    tokio_tungstenite::connect_async_tls_with_config(
                        &url, None, false, Some(conn),
                    ).await
                } else {
                    tokio_tungstenite::connect_async(&url).await
                };

                let ws = match ws_result {
                    Ok((ws, _)) => ws,
                    Err(e) => {
                        let _ = server_tx.send(ServerMsg::Error {
                            msg: format!("Connection failed: {e}"),
                        });
                        return;
                    }
                };

                let (mut ws_tx, mut ws_rx) = ws.split();

                let (outgoing_tx, mut outgoing_rx) =
                    tokio::sync::mpsc::unbounded_channel::<String>();

                tokio::spawn(async move {
                    while let Some(json) = outgoing_rx.recv().await {
                        if ws_tx.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                });

                let outgoing_tx2 = outgoing_tx.clone();
                let poll_handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(20));
                    loop {
                        interval.tick().await;
                        loop {
                            match client_rx.try_recv() {
                                Ok(msg) => {
                                    let json = serde_json::to_string(&msg).unwrap();
                                    if outgoing_tx2.send(json).is_err() { return; }
                                }
                                Err(mpsc::TryRecvError::Empty) => break,
                                Err(mpsc::TryRecvError::Disconnected) => return,
                            }
                        }
                    }
                });

                while let Some(Ok(msg)) = ws_rx.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
                            if server_tx.send(server_msg).is_err() { break; }
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
