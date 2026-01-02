#![forbid(unsafe_code)]

use colored::*;

fn main() {
    println!("{}", "TrinityChain CLI".bright_cyan().bold());
    println!("{}", "----------------".bright_cyan());
    println!();
    println!(
        "{}",
        "This is the main entry point, but most functionality is in separate binaries.".yellow()
    );
    println!(
        "{}",
        "Use 'cargo run --bin <binary_name>' to run a specific command.".yellow()
    );
    println!();
    println!("{}", "Available binaries:".bright_green().underline());
    println!("  - {}", "trinity-node".bright_white());
    println!("  - {}", "trinity-mine-block".bright_white());
    println!("  - {}", "trinity-miner".bright_white());
    println!("  - {}", "trinity-send".bright_white());
    println!("  - {}", "trinity-history".bright_white());
    println!("  - {}", "trinity-balance".bright_white());
    println!("  - {}", "trinity-wallet".bright_white());
    println!("  - {}", "trinity-wallet-backup".bright_white());
    println!("  - {}", "trinity-wallet-restore".bright_white());
    println!("  - {}", "trinity-addressbook".bright_white());
    println!("  - {}", "trinity-guestbook".bright_white());
    println!("  - {}", "trinity-connect".bright_white());
    println!("  - {}", "trinity-server".bright_white());
    println!("  - {}", "trinity-telegram-bot".bright_white());
    println!();
    println!("{}", "Example:".bright_green().underline());
    println!("{}", "  cargo run --bin trinity-node".italic());
}
