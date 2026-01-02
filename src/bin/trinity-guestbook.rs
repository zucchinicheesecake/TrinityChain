#![forbid(unsafe_code)]
use clap::{Parser, Subcommand};
use colored::*;
use std::collections::HashSet;
use trinitychain::cli::load_blockchain_from_config;
use trinitychain::crypto::{address_from_hex, address_from_string, address_to_hex};
use trinitychain::geometry::Coord;
use trinitychain::transaction::{Transaction, TransferTx};
use trinitychain::wallet;

const GUESTBOOK_ADDRESS: &str = "trinity-guestbook-address-00000000000000000";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Signs the guestbook with a message
    Sign {
        /// The message to leave in the guestbook
        message: String,
        /// The name of the wallet to use for signing
        #[arg(long)]
        wallet: Option<String>,
    },
    /// Views the guestbook messages
    View,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Sign { message, wallet } => {
            sign(message, wallet.as_deref()).await?;
        }
        Commands::View => {
            view()?;
        }
    }

    Ok(())
}

async fn sign(message: &str, wallet_name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🖋️  Signing the guestbook...".bright_cyan());

    let (from_wallet, from_address, keypair) = match wallet_name {
        Some(name) => {
            let w = wallet::load_named_wallet(name)?;
            let kp = w.get_keypair()?;
            (w.name.unwrap_or_default(), w.address, kp)
        }
        None => {
            let w = wallet::load_default_wallet()?;
            let kp = w.get_keypair()?;
            ("default".to_string(), w.address, kp)
        }
    };

    println!("Signing with wallet: {}", from_wallet.bright_yellow());

    let from_address_bytes = address_from_hex(&from_address)?;

    let (_config, mut chain) = load_blockchain_from_config()?;

    let mut locked_triangles = HashSet::new();
    if let Ok(mempool_data) = std::fs::read_to_string("mempool.json") {
        if let Ok(txs) = serde_json::from_str::<Vec<Transaction>>(&mempool_data) {
            for tx in txs {
                if let Transaction::Transfer(transfer_tx) = tx {
                    locked_triangles.insert(transfer_tx.input_hash);
                }
            }
        }
    }

    let (input_hash, _input_triangle) = chain
        .state
        .utxo_set
        .iter()
        .find(|(hash, triangle)| {
            triangle.owner == from_address_bytes
                && !locked_triangles.contains(*hash)
                && triangle.effective_value() >= Coord::from_num(0.0001)
        })
        .ok_or("No UTXOs available to pay for the guestbook signing fee.")?;

    let mut tx = TransferTx::new(
        *input_hash,
        address_from_string(GUESTBOOK_ADDRESS),
        from_address_bytes,
        Coord::from_num(0),      // No value transferred to the guestbook address
        Coord::from_num(0.0001), // A small fee to get the transaction mined
        chain.blocks.len() as u64,
    )
    .with_memo(message.to_string())?;

    let message_to_sign = tx.signable_message();
    let signature = keypair.sign(&message_to_sign)?;
    let public_key = keypair.public_key.serialize().to_vec();
    tx.sign(signature.to_vec(), public_key.to_vec());

    let transaction = Transaction::Transfer(tx);
    chain.mempool.add_transaction(transaction.clone())?;

    let all_txs = chain.mempool.get_all_transactions();
    std::fs::write("mempool.json", serde_json::to_string(&all_txs)?)?;

    println!("{}", "Guestbook signed successfully!".bright_green());
    println!("Your message will be on the blockchain soon.");

    Ok(())
}

fn view() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "📖  Viewing Guestbook Entries".bright_cyan());
    println!("{}", "--------------------------".bright_cyan());

    let (_config, chain) = load_blockchain_from_config()?;

    let mut entries_found = false;
    for block in &chain.blocks {
        for tx in &block.transactions {
            if let Transaction::Transfer(transfer_tx) = tx {
                if transfer_tx.new_owner == address_from_string(GUESTBOOK_ADDRESS) {
                    if let Some(memo) = &transfer_tx.memo {
                        entries_found = true;
                        println!(
                            "{} from {}: {}",
                            "•".bright_yellow(),
                            address_to_hex(&transfer_tx.sender).bright_green(),
                            memo
                        );
                    }
                }
            }
        }
    }

    if !entries_found {
        println!("{}", "No guestbook entries found yet.".yellow());
    }

    Ok(())
}
