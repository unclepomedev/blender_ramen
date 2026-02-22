use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

/// Sends the generated Python script to the Blender Live-Link server.
pub fn send_to_blender(script: &str) {
    println!("🍜 Blender Ramen: Sending script via Live-Link...");

    let target = "127.0.0.1:8080".parse().unwrap();
    match TcpStream::connect_timeout(&target, Duration::from_secs(2)) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(script.as_bytes()) {
                eprintln!("❌ Failed to transfer the script: {}", e);
            } else {
                println!("✅ Live-Link successful! Transferred the node tree to Blender!");
            }
        }
        Err(e) => {
            eprintln!("❌ Could not connect to Blender: {}", e);
            eprintln!("💡 Hint: Is the Live-Link server (Python script) running in Blender?");
        }
    }
}
