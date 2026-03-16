use cheshire_chess::server::start_central_server;

fn main() {
    println!("Starting Cheshire Chess game server on port 7880...");
    start_central_server();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
