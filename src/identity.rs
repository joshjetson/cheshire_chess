#![allow(dead_code)]

use std::fs;
use std::path::Path;

/// Get or create a persistent client identity hash.
/// Stored in data/client_id as a 64-char hex string.
pub fn get_or_create_client_id(data_dir: &Path) -> String {
    let id_path = data_dir.join("client_id");

    // Try to read existing ID
    if let Ok(id) = fs::read_to_string(&id_path) {
        let id = id.trim().to_string();
        if id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) {
            return id;
        }
    }

    // Generate new ID from random bytes
    let id = generate_id();

    // Save it
    let _ = fs::create_dir_all(data_dir);
    let _ = fs::write(&id_path, &id);

    id
}

fn generate_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".into());

    let pid = std::process::id();

    // Hash multiple entropy sources
    let mut hasher = DefaultHasher::new();
    timestamp.hash(&mut hasher);
    hostname.hash(&mut hasher);
    pid.hash(&mut hasher);
    let h1 = hasher.finish();

    // Second hash with different seed
    let mut hasher2 = DefaultHasher::new();
    h1.hash(&mut hasher2);
    (timestamp ^ 0xDEADBEEF).hash(&mut hasher2);
    hostname.len().hash(&mut hasher2);
    let h2 = hasher2.finish();

    // Third and fourth
    let mut hasher3 = DefaultHasher::new();
    h2.hash(&mut hasher3);
    (pid as u64 ^ timestamp as u64).hash(&mut hasher3);
    let h3 = hasher3.finish();

    let mut hasher4 = DefaultHasher::new();
    h3.hash(&mut hasher4);
    h1.hash(&mut hasher4);
    let h4 = hasher4.finish();

    format!("{h1:016x}{h2:016x}{h3:016x}{h4:016x}")
}

/// Validate that a string looks like a valid client ID.
pub fn is_valid_client_id(id: &str) -> bool {
    id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit())
}
