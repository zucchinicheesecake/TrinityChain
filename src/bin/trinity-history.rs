#![forbid(unsafe_code)]
//! View transaction history for your wallet - Beautiful edition!

use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::Color as TableColor;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use trinitychain::cli::load_blockchain_from_config;
use trinitychain::crypto::{address_from_hex, address_to_hex};
use trinitychain::transaction::Transaction;

const LOGO: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║     ████████╗██████╗ ██╗███╗   ██╗██╗████████╗██╗   ██╗      ║
║     ╚══██╔══╝██╔══██╗██║████╗  ██║██║╚══██╔══╝╚██╗ ██╔╝      ║
║        ██║   ██████╔╝██║██╔██╗ ██║██║   ██║    ╚████╔╝       ║
║        ██║   ██╔══██╗██║██║╚██╗██║██║   ██║     ╚██╔╝        ║
║        ██║   ██║  ██║██║██║ ╚████║██║   ██║      ██║         ║
║        ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝   ╚═╝      ╚═╝         ║
║                 🔺 Transaction History 🔺                      ║
╚═══════════════════════════════════════════════════════════════╝
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", LOGO.bright_magenta());

    let home = std::env::var("HOME")?;
    let wallet_file = format!("{}/.trinitychain/wallet.json", home);

    let wallet_content = std::fs::read_to_string(&wallet_file).map_err(|e| {
        eprintln!("{}", "╔══════════════════════════════════════════╗".red());
        eprintln!(
            "{}",
            "║         ❌ Wallet Not Found!            ║".red().bold()
        );
        eprintln!("{}", "╚══════════════════════════════════════════╝".red());
        eprintln!();
        eprintln!("{}", "💡 Run 'wallet new' to create a wallet".yellow());
        format!("No wallet found at {}: {}", wallet_file, e)
    })?;

    let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)
        .map_err(|e| format!("Failed to parse wallet: {}", e))?;

    let my_address = wallet_data["address"]
        .as_str()
        .ok_or("Wallet address not found in wallet file")?;

    let my_address_bytes = address_from_hex(my_address)?;

    let (_config, chain) = load_blockchain_from_config()?;

    let addr_display = if my_address.len() > 40 {
        format!(
            "{}...{}",
            &my_address[..20],
            &my_address[my_address.len() - 16..]
        )
    } else {
        my_address.to_string()
    };

    println!(
        "{}",
        "┌─────────────────────────────────────────────────────────────┐".bright_cyan()
    );
    println!(
        "{}",
        "│                  📜 TRANSACTION HISTORY                     │"
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "└─────────────────────────────────────────────────────────────┘".bright_cyan()
    );
    println!();
    println!("{}", format!("📍 Address: {}", addr_display).cyan());
    println!();

    let mut tx_count = 0;
    let mut received_count = 0;
    let mut sent_count = 0;
    let mut mining_count = 0;

    struct TxRecord {
        block_height: u64,
        tx_type: String,
        direction: String,
        details: String,
        timestamp: i64,
        color: TableColor,
    }

    let mut transactions: Vec<TxRecord> = Vec::new();

    // Iterate through all blocks
    for block in &chain.blocks {
        for tx in &block.transactions {
            match tx {
                Transaction::Transfer(transfer_tx) => {
                    let is_sender = transfer_tx.sender == my_address_bytes;
                    let is_receiver = transfer_tx.new_owner == my_address_bytes;

                    if is_sender || is_receiver {
                        tx_count += 1;

                        let (direction, color) = if is_sender && is_receiver {
                            ("↔️  Self".to_string(), TableColor::Yellow)
                        } else if is_sender {
                            sent_count += 1;
                            ("📤 Sent".to_string(), TableColor::Red)
                        } else {
                            received_count += 1;
                            ("📥 Received".to_string(), TableColor::Green)
                        };

                        let hash_hex = hex::encode(transfer_tx.input_hash);
                        let hash_short = if hash_hex.len() > 16 {
                            format!("{}...", &hash_hex[..13])
                        } else {
                            hash_hex
                        };

                        let other_party = if is_sender {
                            let addr_hex = address_to_hex(&transfer_tx.new_owner);
                            if addr_hex.len() > 20 {
                                format!(
                                    "To: {}...{}",
                                    &addr_hex[..8],
                                    &addr_hex[addr_hex.len() - 8..]
                                )
                            } else {
                                format!("To: {}", addr_hex)
                            }
                        } else {
                            let addr_hex = address_to_hex(&transfer_tx.sender);
                            if addr_hex.len() > 20 {
                                format!(
                                    "From: {}...{}",
                                    &addr_hex[..8],
                                    &addr_hex[addr_hex.len() - 8..]
                                )
                            } else {
                                format!("From: {}", addr_hex)
                            }
                        };

                        let memo_str = if let Some(memo) = &transfer_tx.memo {
                            if memo.len() > 20 {
                                format!(" | \"{}...\"", &memo[..17])
                            } else {
                                format!(" | \"{}\"", memo)
                            }
                        } else {
                            String::new()
                        };

                        transactions.push(TxRecord {
                            block_height: block.header.height,
                            tx_type: "Transfer".to_string(),
                            direction,
                            details: format!("{} | {}{}", hash_short, other_party, memo_str),
                            timestamp: block.header.timestamp as i64,
                            color,
                        });
                    }
                }
                Transaction::Coinbase(coinbase_tx) => {
                    if coinbase_tx.beneficiary_address == my_address_bytes {
                        tx_count += 1;
                        received_count += 1;
                        mining_count += 1;

                        transactions.push(TxRecord {
                            block_height: block.header.height,
                            tx_type: "Mining".to_string(),
                            direction: "⛏️  Reward".to_string(),
                            details: format!("Area: {}", coinbase_tx.reward_area),
                            timestamp: block.header.timestamp as i64,
                            color: TableColor::Cyan,
                        });
                    }
                }
                Transaction::Subdivision(sub_tx) => {
                    if sub_tx.owner_address == my_address_bytes {
                        tx_count += 1;

                        let hash_hex = hex::encode(sub_tx.parent_hash);
                        let hash_short = if hash_hex.len() > 16 {
                            format!("{}...", &hash_hex[..13])
                        } else {
                            hash_hex
                        };

                        transactions.push(TxRecord {
                            block_height: block.header.height,
                            tx_type: "Subdivision".to_string(),
                            direction: "✂️  Split".to_string(),
                            details: format!("{} → {} children", hash_short, sub_tx.children.len()),
                            timestamp: block.header.timestamp as i64,
                            color: TableColor::Magenta,
                        });
                    }
                }
            }
        }
    }

    if transactions.is_empty() {
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════════╗".yellow()
        );
        println!(
            "{}",
            "║              📭 No Transactions Found                    ║".yellow()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════╣".yellow()
        );
        println!(
            "{}",
            "║  No transaction history yet. Start using your wallet!   ║".yellow()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════╝".yellow()
        );
        println!();
        return Ok(());
    }

    // Reverse to show newest first
    transactions.reverse();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Block")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Type")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Direction")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Details")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Date")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
        ]);

    for tx in &transactions {
        table.add_row(vec![
            Cell::new(format!("#{}", tx.block_height)).fg(TableColor::White),
            Cell::new(&tx.tx_type).fg(tx.color),
            Cell::new(&tx.direction).fg(tx.color),
            Cell::new(&tx.details).fg(TableColor::White),
            Cell::new(format_timestamp_short(tx.timestamp)).fg(TableColor::Grey),
        ]);
    }

    println!("{}", table);
    println!();

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".bright_blue()
    );
    println!(
        "{}",
        "║                    📊 TRANSACTION SUMMARY                ║"
            .bright_blue()
            .bold()
    );
    println!(
        "{}",
        "╠══════════════════════════════════════════════════════════╣".bright_blue()
    );
    println!(
        "{}",
        format!("║  📝 Total Transactions: {:<33} ║", tx_count).blue()
    );
    println!(
        "{}",
        format!("║  📥 Received: {:<43} ║", received_count).green()
    );
    println!("{}", format!("║  📤 Sent: {:<47} ║", sent_count).red());
    println!(
        "{}",
        format!("║  ⛏️  Mining Rewards: {:<36} ║", mining_count).cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".bright_blue()
    );
    println!();

    Ok(())
}

fn format_timestamp_short(timestamp: i64) -> String {
    use chrono::DateTime;

    if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
        dt.format("%m/%d %H:%M").to_string()
    } else {
        "Invalid".to_string()
    }
}
