#![forbid(unsafe_code)]
//! Network node for TrinityChain - TUI Edition

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use trinitychain::blockchain::Blockchain;
use trinitychain::config::load_config;
use trinitychain::persistence::Database;

#[derive(Clone)]
struct NodeStats {
    chain_height: u64,
    uptime_secs: u64,
    status: String,
    last_block_hash: String,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            chain_height: 0,
            uptime_secs: 0,
            status: "Initializing...".to_string(),
            last_block_hash: "N/A".to_string(),
        }
    }
}

// UI drawing functions remain the same...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let db_path = config.database.path;
    let p2p_port = config.network.p2p_port;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db = Database::open(&db_path).expect("Failed to open database");
    let blockchain = db
        .load_blockchain()
        .unwrap_or_else(|_| Blockchain::new([0; 32], 1).expect("Failed to create new blockchain"));

    let blockchain = Arc::new(tokio::sync::RwLock::new(blockchain));
    let stats = Arc::new(tokio::sync::Mutex::new(NodeStats::default()));
    let start_time = Instant::now();

    // Start P2P networking in background
    let _p2p_task = tokio::spawn(async move {
        println!("🌐 P2P Server listening on port {}", p2p_port);
        // Network initialization would happen here
    });

    // Main UI loop
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        {
            let mut s = stats.lock().await;
            s.status = "Running".to_string();
            s.uptime_secs = start_time.elapsed().as_secs();

            let bc = blockchain.read().await;
            s.chain_height = bc.blocks.len() as u64;
            if let Some(last_block) = bc.blocks.last() {
                s.last_block_hash = hex::encode(last_block.hash());
            }
        }

        let _stats_clone = stats.lock().await.clone();
        terminal.draw(|_f| {
            // Minimal UI - can be expanded with ratatui widgets
        })?;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
