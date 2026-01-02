#![forbid(unsafe_code)]
use std::env;
use std::time::Instant;
use trinitychain::blockchain::{Block, Blockchain};
use trinitychain::crypto::{address_from_hex, address_to_hex};
use trinitychain::miner::mine_block;
use trinitychain::persistence::Database;
use trinitychain::transaction::{CoinbaseTx, Transaction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <address>", args[0]);
        return Ok(());
    }
    let address_hex = &args[1];
    let address = address_from_hex(address_hex)?;

    let db = Database::open("trinitychain.db")?;
    let mut chain = db.load_blockchain().unwrap_or_else(|_| {
        println!("⛓️  No chain found – creating genesis block...\n");
        Blockchain::new(address, 1).unwrap()
    });

    let last_block = chain.blocks.last().cloned().unwrap();
    let new_height = last_block.header.height + 1;

    let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
        reward_area: trinitychain::geometry::Coord::from_num(1000),
        beneficiary_address: address,
        nonce: new_height,
    });

    let transactions = vec![coinbase_tx];

    let mut new_block = Block::new(
        new_height,
        last_block.hash(),
        chain.difficulty,
        transactions,
    );

    if new_block.header.timestamp <= last_block.header.timestamp {
        new_block.header.timestamp = last_block.header.timestamp + 1;
    }

    // Print mining header
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!(
        "║              ⛏️  MINING BLOCK {}                          ║",
        new_height
    );
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let start_time = Instant::now();
    let new_block = mine_block(new_block)?;
    let elapsed = start_time.elapsed();

    chain.apply_block(new_block.clone())?;
    db.save_blockchain_state(&new_block, &chain.state, chain.difficulty as u64)?;

    let block_hash = hex::encode(new_block.hash());
    let prev_hash = hex::encode(new_block.header.previous_hash);
    let beneficiary = address_to_hex(&address);

    // Find the reward triangle in the UTXO set
    let reward_triangle = chain
        .state
        .utxo_set
        .values()
        .find(|t| t.owner == address && t.effective_value().to_num::<f64>() >= 999.0)
        .cloned();

    // Print enhanced mining results
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                      ✅ BLOCK MINED!                         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    println!(
        "┌──────────────────────────────────── BLOCK METADATA ───────────────────────────────────┐"
    );
    println!("│ Height:              #{:<66} │", new_height);
    println!("│ Hash:                {} │", block_hash);
    println!("│ Previous:            {} │", prev_hash);
    println!(
        "│ Timestamp:           {:<66} │",
        new_block.header.timestamp
    );
    println!(
        "│ Difficulty:          {:<66} │",
        new_block.header.difficulty
    );
    println!("│ Nonce:               {:<66} │", new_block.header.nonce);
    println!(
        "│ Mining Time:         {:.3} seconds{:<57} │",
        elapsed.as_secs_f64(),
        ""
    );
    println!("└────────────────────────────────────────────────────────────────────────────────────────┘\n");

    println!(
        "┌──────────────────────────────── 🔺 REWARD TRIANGLE ───────────────────────────────────┐"
    );
    println!(
        "│ Reward Type:         Coinbase Transaction                                             │"
    );
    println!(
        "│ Amount:              1000.000000 TRC (geometric area units)                          │"
    );
    println!("│ Beneficiary:         {} │", beneficiary);

    if let Some(triangle) = reward_triangle {
        let tri_hash = hex::encode(triangle.hash());
        let area = triangle.effective_value().to_num::<f64>();
        println!("│ Triangle Hash:       {} │", tri_hash);
        println!(
            "│ Triangle Area:       {:.6}                                          │",
            area
        );
        println!(
            "│ Vertices:            A({}, {}), B({}, {}), C({}, {})  │",
            triangle.a.x.to_num::<f64>(),
            triangle.a.y.to_num::<f64>(),
            triangle.b.x.to_num::<f64>(),
            triangle.b.y.to_num::<f64>(),
            triangle.c.x.to_num::<f64>(),
            triangle.c.y.to_num::<f64>()
        );
    } else {
        println!("│ Triangle Hash:       Queued for next confirmation                                   │");
        println!(
            "│ Status:              NEW - Created this block                                     │"
        );
    }
    println!("└────────────────────────────────────────────────────────────────────────────────────────┘\n");

    println!("┌────────────────────────────────── 📊 NETWORK STATE ─────────────────────────────────────┐");
    println!("│ Total Blocks:        {:<65} │", chain.blocks.len());
    println!(
        "│ Total UTXOs:         {:<65} │",
        chain.state.utxo_set.len()
    );
    println!(
        "│ Transactions:        {:<65} │",
        new_block.transactions.len()
    );
    println!("│ Chain Difficulty:    {:<65} │", chain.difficulty);
    println!(
        "│ Your Balance:        {} TRC (from {} triangles)                    │",
        chain.state.get_balance(&address).to_num::<f64>(),
        chain
            .state
            .utxo_set
            .values()
            .filter(|t| t.owner == address)
            .count()
    );
    println!("└────────────────────────────────────────────────────────────────────────────────────────┘\n");

    Ok(())
}
