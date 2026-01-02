#![forbid(unsafe_code)]
use sha2::{Digest, Sha256};
use trinitychain::crypto::Address;
use trinitychain::geometry::Coord;
use trinitychain::persistence::Database;

fn address_from_string(s: &str) -> Address {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hasher.finalize().into()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Address to check
    let addr = address_from_string("miner");

    let db = Database::open("trinitychain.db")?;
    let chain = db.load_blockchain()?;

    let balance: Coord = chain.state.get_balance(&addr);
    let balance_f: f64 = balance.to_num();

    println!("Address: miner");
    println!(
        "Chain height: {}",
        chain.blocks.last().map(|b| b.header.height).unwrap_or(0)
    );
    println!("Balance (raw Coord): {:?}", balance);
    println!("Balance (as f64): {:.6}", balance_f);

    Ok(())
}
