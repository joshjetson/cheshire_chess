use cheshire_chess::server::start_server;

fn main() {
    println!("Starting Cheshire Chess dedicated server...");
    start_server();
    // Keep main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
