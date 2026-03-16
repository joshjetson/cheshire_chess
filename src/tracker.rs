use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const TRACKER_HOST: &str = "chess.virtualraremedia.com";
const TRACKER_PORT: u16 = 7879;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteServer {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub players: u32,
}

fn connect_tracker() -> Option<TcpStream> {
    let addr = format!("{TRACKER_HOST}:{TRACKER_PORT}");
    let resolved = addr.to_socket_addrs().ok()?.next()?;
    TcpStream::connect_timeout(&resolved, Duration::from_secs(3)).ok()
}

/// Fetch the server list from the tracker. Blocking.
pub fn fetch_servers() -> Vec<RemoteServer> {
    let mut stream = match connect_tracker() {
        Some(s) => s,
        None => return Vec::new(),
    };
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

    let request = format!(
        "GET /servers HTTP/1.1\r\nHost: {TRACKER_HOST}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return Vec::new();
    }

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    let body = response.split("\r\n\r\n").nth(1).unwrap_or("[]");
    serde_json::from_str(body).unwrap_or_default()
}

/// Register this server with the tracker. Blocking.
pub fn register(host: &str, port: u16, name: &str, players: u32) {
    let mut stream = match connect_tracker() {
        Some(s) => s,
        None => return,
    };

    let body = serde_json::to_string(&serde_json::json!({
        "host": host,
        "port": port,
        "name": name,
        "players": players,
    })).unwrap();

    let request = format!(
        "POST /register HTTP/1.1\r\nHost: {TRACKER_HOST}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    let _ = stream.write_all(request.as_bytes());
}

/// Get this machine's public IP by asking the tracker what it sees.
/// Falls back to hostname.
pub fn get_public_ip() -> String {
    // Simple: ask an external service for our public IP
    let addrs = format!("api.ipify.org:80").to_socket_addrs();
    if let Ok(mut resolved) = addrs {
        if let Some(addr) = resolved.next() {
            if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
                stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
                let req = "GET / HTTP/1.1\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n";
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut resp = String::new();
                    let _ = stream.read_to_string(&mut resp);
                    if let Some(body) = resp.split("\r\n\r\n").nth(1) {
                        let ip = body.trim().to_string();
                        if !ip.is_empty() {
                            return ip;
                        }
                    }
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

/// Start a background heartbeat thread that re-registers every 30 seconds.
pub fn start_heartbeat(host: String, port: u16, name: String) -> mpsc::Sender<u32> {
    let (tx, rx) = mpsc::channel::<u32>();

    thread::spawn(move || {
        let mut players = 0u32;
        loop {
            while let Ok(count) = rx.try_recv() {
                players = count;
            }
            register(&host, port, &name, players);
            thread::sleep(Duration::from_secs(30));
        }
    });

    tx
}
