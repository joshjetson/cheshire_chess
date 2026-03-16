use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

const TTL_SECS: u64 = 60;
const LISTEN: &str = "0.0.0.0:7879";

#[derive(Clone, Serialize)]
struct ServerEntry {
    host: String,
    port: u16,
    name: String,
    players: u32,
    #[serde(skip)]
    last_seen: Instant,
}

#[derive(Deserialize)]
struct RegisterReq {
    host: String,
    port: u16,
    name: String,
    players: u32,
}

type State = Arc<Mutex<HashMap<String, ServerEntry>>>;

#[tokio::main]
async fn main() {
    let state: State = Arc::new(Mutex::new(HashMap::new()));

    // Prune expired servers every 15s
    let prune_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let mut map = prune_state.lock().await;
            let now = Instant::now();
            map.retain(|_, v| now.duration_since(v.last_seen).as_secs() < TTL_SECS);
        }
    });

    let listener = TcpListener::bind(LISTEN).await.expect("Failed to bind tracker");
    println!("Cheshire Chess tracker on {LISTEN}");

    loop {
        let (mut stream, _peer) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };

        let state = state.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let n = match stream.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");

            if first_line.starts_with("GET /servers") {
                let map = state.lock().await;
                let servers: Vec<&ServerEntry> = map.values().collect();
                let json = serde_json::to_string(&servers).unwrap_or_else(|_| "[]".into());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                    json.len(), json
                );
                let _ = stream.write_all(response.as_bytes()).await;
            } else if first_line.starts_with("POST /register") {
                // Find the JSON body after the \r\n\r\n
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
                match serde_json::from_str::<RegisterReq>(body) {
                    Ok(req) => {
                        let key = format!("{}:{}", req.host, req.port);
                        let entry = ServerEntry {
                            host: req.host,
                            port: req.port,
                            name: req.name,
                            players: req.players,
                            last_seen: Instant::now(),
                        };
                        state.lock().await.insert(key, entry);
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                    Err(_) => {
                        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 3\r\n\r\nbad";
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                }
            } else if first_line.starts_with("DELETE /register") {
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
                if let Ok(req) = serde_json::from_str::<RegisterReq>(body) {
                    let key = format!("{}:{}", req.host, req.port);
                    state.lock().await.remove(&key);
                }
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                let _ = stream.write_all(response.as_bytes()).await;
            } else {
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });
    }
}
