use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

use cheshire_chess::puzzle::PuzzleIndex;

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

struct TrackerState {
    servers: HashMap<String, ServerEntry>,
    puzzle_index: Option<PuzzleIndex>,
}

type State = Arc<Mutex<TrackerState>>;

fn parse_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&')
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next()?;
            if k == key { Some(v) } else { None }
        })
}

fn respond_json(json: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
        json.len(), json
    )
}

fn respond_error(code: u16, msg: &str) -> String {
    format!("HTTP/1.1 {code} Error\r\nContent-Length: {}\r\n\r\n{msg}", msg.len())
}

#[tokio::main]
async fn main() {
    // Load puzzle index if CSV is available
    let puzzle_csv = std::env::var("PUZZLE_CSV")
        .unwrap_or_else(|_| "/data/lichess_puzzles.csv".to_string());

    let puzzle_index = if std::path::Path::new(&puzzle_csv).exists() {
        match PuzzleIndex::build(std::path::Path::new(&puzzle_csv)) {
            Ok(idx) => {
                println!("Puzzle index built: {} puzzles", idx.total);
                Some(idx)
            }
            Err(e) => {
                eprintln!("Failed to build puzzle index: {e}");
                None
            }
        }
    } else {
        println!("No puzzle CSV at {puzzle_csv}, puzzle API disabled");
        None
    };

    let state: State = Arc::new(Mutex::new(TrackerState {
        servers: HashMap::new(),
        puzzle_index,
    }));

    // Prune expired servers every 15s
    let prune_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let mut st = prune_state.lock().await;
            let now = Instant::now();
            st.servers.retain(|_, v| now.duration_since(v.last_seen).as_secs() < TTL_SECS);
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
            let mut buf = vec![0u8; 8192];
            let n = match stream.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");

            // Extract client ID header
            let client_id = request.lines()
                .find(|l| l.to_lowercase().starts_with("x-cheshire-id:"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string());

            // ── GET /servers ───────────────────────────────────────
            if first_line.starts_with("GET /servers") {
                let st = state.lock().await;
                let servers: Vec<&ServerEntry> = st.servers.values().collect();
                let json = serde_json::to_string(&servers).unwrap_or_else(|_| "[]".into());
                let _ = stream.write_all(respond_json(&json).as_bytes()).await;

            // ── GET /puzzles ───────────────────────────────────────
            } else if first_line.starts_with("GET /puzzles") {
                // Validate client ID
                let valid_id = client_id
                    .as_ref()
                    .map(|id| id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()))
                    .unwrap_or(false);

                if !valid_id {
                    let _ = stream.write_all(respond_error(403, "invalid client").as_bytes()).await;
                    return;
                }

                let st = state.lock().await;
                let index = match &st.puzzle_index {
                    Some(idx) => idx,
                    None => {
                        let _ = stream.write_all(respond_error(503, "puzzles not available").as_bytes()).await;
                        return;
                    }
                };

                // Parse query: /puzzles?theme=fork&max_rating=2000&offset=0&limit=50
                let query = first_line.split('?').nth(1).unwrap_or("");
                let theme = parse_query_param(query, "theme").unwrap_or("fork");
                let max_rating: u16 = parse_query_param(query, "max_rating")
                    .and_then(|v| v.parse().ok()).unwrap_or(2500);
                let limit: usize = parse_query_param(query, "limit")
                    .and_then(|v| v.parse().ok()).unwrap_or(50).min(100);
                let offset: usize = parse_query_param(query, "offset")
                    .and_then(|v| v.parse().ok()).unwrap_or(0);

                // Load puzzles using the index
                match index.load_theme_with_offset(theme, Some(max_rating), limit, offset) {
                    Ok(puzzles) => {
                        let json = serde_json::to_string(&puzzles).unwrap_or_else(|_| "[]".into());
                        let _ = stream.write_all(respond_json(&json).as_bytes()).await;
                    }
                    Err(_) => {
                        let _ = stream.write_all(respond_json("[]").as_bytes()).await;
                    }
                }

            // ── GET /themes ────────────────────────────────────────
            } else if first_line.starts_with("GET /themes") {
                let st = state.lock().await;
                if let Some(ref index) = st.puzzle_index {
                    let json = serde_json::to_string(&index.theme_counts).unwrap_or_else(|_| "[]".into());
                    let _ = stream.write_all(respond_json(&json).as_bytes()).await;
                } else {
                    let _ = stream.write_all(respond_json("[]").as_bytes()).await;
                }

            // ── POST /register ─────────────────────────────────────
            } else if first_line.starts_with("POST /register") {
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
                match serde_json::from_str::<RegisterReq>(body) {
                    Ok(req) => {
                        let key = format!("{}:{}", req.host, req.port);
                        let entry = ServerEntry {
                            host: req.host, port: req.port, name: req.name,
                            players: req.players, last_seen: Instant::now(),
                        };
                        state.lock().await.servers.insert(key, entry);
                        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
                    }
                    Err(_) => {
                        let _ = stream.write_all(respond_error(400, "bad request").as_bytes()).await;
                    }
                }
            } else if first_line.starts_with("DELETE /register") {
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
                if let Ok(req) = serde_json::from_str::<RegisterReq>(body) {
                    let key = format!("{}:{}", req.host, req.port);
                    state.lock().await.servers.remove(&key);
                }
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
            } else if first_line.starts_with("GET /version") {
                let version = env!("CARGO_PKG_VERSION");
                let _ = stream.write_all(respond_json(&format!("\"{version}\"")).as_bytes()).await;
            } else {
                let _ = stream.write_all(respond_error(404, "not found").as_bytes()).await;
            }
        });
    }
}
