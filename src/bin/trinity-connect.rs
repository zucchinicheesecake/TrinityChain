#![forbid(unsafe_code)]
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use trinitychain::blockchain::Blockchain;
use trinitychain::config::load_config;
use trinitychain::network::NetworkNode;
use trinitychain::persistence::Database;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "peer" => {
            if args.len() < 3 {
                eprintln!("Usage: trinity-connect peer <ip:port>");
                return;
            }
            connect_peer(&args[2]).await;
        }
        "info" => show_info().await,
        _ => print_usage(),
    }
}

async fn connect_peer(addr: &str) {
    println!("🔗 Connecting to peer: {}", addr);
    let config = load_config().expect("Failed to load config");
    let db = Database::open(&config.database.path).expect("DB open failed");
    let blockchain = db
        .load_blockchain()
        .unwrap_or_else(|_| Blockchain::new([0; 32], 1).expect("Failed to create new blockchain"));
    let node = Arc::new(NetworkNode::new(Arc::new(RwLock::new(blockchain))));

    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        eprintln!("❌ Format: IP:PORT");
        return;
    }

    let host = parts[0].to_string();
    let port = parts[1].parse::<u16>().unwrap_or(8334);

    match node.clone().connect_peer(host, port).await {
        Ok(_) => println!("✅ Connected! Syncing..."),
        Err(e) => eprintln!("❌ Failed: {}", e),
    }
}

async fn show_info() {
    println!("🔺 TrinityChain Network Info");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if let Ok(output) = std::process::Command::new("sh")
        .arg("-c")
        .arg("ip addr show | grep 'inet ' | awk '{print $2}' | cut -d/ -f1")
        .output()
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() {
            println!("📡 Your IP: {}", ip);
            println!("\n💡 Share this with peers:");
            println!("   {}:8334", ip);
        }
    }
}

fn print_usage() {
    println!("Usage:");
    println!("  trinity-connect peer <ip:port>  - Connect to peer");
    println!("  trinity-connect info            - Show your IP");
}
