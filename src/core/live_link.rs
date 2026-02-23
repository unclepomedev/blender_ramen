use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

const LIVE_LINK_ADDR: &str = "127.0.0.1:8080";

/// Sends the generated Python script to the Blender Live-Link server.
pub fn send_to_blender(script: &str) {
    println!("🍜 Blender Ramen: Sending script via Live-Link...");

    let target = LIVE_LINK_ADDR.parse().unwrap();
    match TcpStream::connect_timeout(&target, Duration::from_secs(2)) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(script.as_bytes()) {
                eprintln!("❌ Failed to transfer the script: {}", e);
                return;
            }
            let _ = stream.shutdown(Shutdown::Write);
            stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() {
                if response.starts_with("ERROR") {
                    eprintln!("❌ Python Execution Failed in Blender:\n{}", response);
                } else {
                    println!("✅ Live-Link successful! Transferred the node tree to Blender!");
                }
            } else {
                eprintln!("⚠️ Script sent, but failed to read response from Blender.");
            }
        }
        Err(e) => {
            eprintln!("❌ Could not connect to Blender: {}", e);
            eprintln!("💡 Hint: Is the Live-Link server (Python script) running in Blender?");
        }
    }
}
