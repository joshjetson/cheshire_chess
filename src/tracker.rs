use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const TRACKER_URL: &str = "chess.virtualraremedia.com";
const TRACKER_PORT: u16 = 443; // Caddy will reverse-proxy to 7879

// For direct connection (non-TLS) during development
const TRACKER_DIRECT_HOST: &str = "chess.virtualraremedia.com";
const TRACKER_DIRECT_PORT: u16 = 7879;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteServer {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub players: u32,
}

/// Fetch the server list from the tracker. Blocking.
pub fn fetch_servers() -> Vec<RemoteServer> {
    let request = format!(
        "GET /servers HTTP/1.1\r\nHost: {TRACKER_DIRECT_HOST}\r\nConnection: close\r\n\r\n"
    );
    let addr = format!("{TRACKER_DIRECT_HOST}:{TRACKER_DIRECT_PORT}");
    let mut stream = match TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| "127.0.0.1:7879".parse().unwrap()),
        Duration::from_secs(3),
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    if stream.write_all(request.as_bytes()).is_err() {
        return Vec::new();
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() && response.is_empty() {
        return Vec::new();
    }

    // Parse body after \r\n\r\n
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("[]");
    serde_json::from_str(body).unwrap_or_default()
}

/// Register this server with the tracker. Blocking.
pub fn register(host: &str, port: u16, name: &str, players: u32) {
    let body = serde_json::to_string(&serde_json::json!({
        "host": host,
        "port": port,
        "name": name,
        "players": players,
    })).unwrap();

    let request = format!(
        "POST /register HTTP/1.1\r\nHost: {TRACKER_DIRECT_HOST}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );

    let addr = format!("{TRACKER_DIRECT_HOST}:{TRACKER_DIRECT_PORT}");
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| "127.0.0.1:7879".parse().unwrap()),
        Duration::from_secs(3),
    ) {
        let _ = stream.write_all(request.as_bytes());
    }
}

/// Start a background heartbeat thread that re-registers every 30 seconds.
/// Returns a channel sender that can be used to update player count.
pub fn start_heartbeat(host: String, port: u16, name: String) -> mpsc::Sender<u32> {
    let (tx, rx) = mpsc::channel::<u32>();

    thread::spawn(move || {
        let mut players = 0u32;
        loop {
            // Check for updated player count
            while let Ok(count) = rx.try_recv() {
                players = count;
            }
            register(&host, port, &name, players);
            thread::sleep(Duration::from_secs(30));
        }
    });

    tx
}
