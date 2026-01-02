//! Integration tests for wallet creation and transaction handling

use tempfile::TempDir;
use trinitychain::blockchain::Blockchain;
use trinitychain::geometry::Coord;
use trinitychain::transaction::{Transaction, TransferTx};
use trinitychain::wallet::Wallet;

/// Helper to create a test wallet
fn create_test_wallet(name: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
    Ok(Wallet::new(Some(name.to_string()))?)
}

/// Helper to get test directory
fn get_test_dir() -> Result<TempDir, Box<dyn std::error::Error>> {
    Ok(TempDir::new()?)
}

#[test]
fn test_wallet_creation() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = create_test_wallet("test_wallet")?;

    // Verify wallet has required fields
    assert_eq!(wallet.name, Some("test_wallet".to_string()));
    assert!(!wallet.address.is_empty());
    assert!(!wallet.secret_key_hex.is_empty());
    assert!(!wallet.created.is_empty());

    // Verify address is 64 hex characters (32 bytes)
    assert_eq!(wallet.address.len(), 64);
    assert!(wallet.address.chars().all(|c| c.is_ascii_hexdigit()));

    Ok(())
}

#[test]
fn test_create_two_wallets() -> Result<(), Box<dyn std::error::Error>> {
    let alice = create_test_wallet("alice")?;
    let bob = create_test_wallet("bob")?;

    // Verify both wallets are created
    assert_eq!(alice.name, Some("alice".to_string()));
    assert_eq!(bob.name, Some("bob".to_string()));

    // Verify they have different addresses (with very high probability)
    assert_ne!(alice.address, bob.address);

    // Verify they have different secret keys
    assert_ne!(alice.secret_key_hex, bob.secret_key_hex);

    Ok(())
}

#[test]
fn test_wallet_persistence() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = get_test_dir()?;
    let wallet_path = temp_dir.path().join("wallet.json");

    // Create and save wallet
    let original_wallet = create_test_wallet("persistent")?;
    original_wallet.save(&wallet_path)?;

    // Verify file exists
    assert!(wallet_path.exists());

    // Load wallet back
    let loaded_wallet = Wallet::load(&wallet_path)?;

    // Verify all fields match
    assert_eq!(original_wallet.address, loaded_wallet.address);
    assert_eq!(original_wallet.name, loaded_wallet.name);
    assert_eq!(original_wallet.secret_key_hex, loaded_wallet.secret_key_hex);
    assert_eq!(original_wallet.created, loaded_wallet.created);

    Ok(())
}

#[test]
fn test_wallet_keypair_derivation() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = create_test_wallet("keypair_test")?;

    // Get keypair from wallet
    let keypair = wallet.get_keypair()?;

    // Verify keypair address matches wallet address
    let keypair_address = hex::encode(keypair.address());
    assert_eq!(wallet.address, keypair_address);

    Ok(())
}

#[test]
fn test_blockchain_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let alice = create_test_wallet("miner")?;
    let mut alice_addr = [0u8; 32];
    hex::decode_to_slice(&alice.address, &mut alice_addr)?;

    // Create blockchain
    let blockchain = Blockchain::new(alice_addr, 4)?;

    // Verify blockchain has genesis block
    assert!(blockchain.blocks.len() > 0);
    assert_eq!(blockchain.blocks[0].header.height, 0);

    Ok(())
}

#[test]
fn test_transfer_transaction_creation() -> Result<(), Box<dyn std::error::Error>> {
    let alice = create_test_wallet("sender")?;
    let bob = create_test_wallet("recipient")?;

    // Parse addresses
    let mut alice_addr = [0u8; 32];
    let mut bob_addr = [0u8; 32];
    hex::decode_to_slice(&alice.address, &mut alice_addr)?;
    hex::decode_to_slice(&bob.address, &mut bob_addr)?;

    // Create transfer transaction
    let transfer = TransferTx {
        input_hash: [1u8; 32],
        new_owner: bob_addr,
        sender: alice_addr,
        amount: Coord::from_num(100),
        fee_area: Coord::from_num(5),
        memo: Some("Test transfer".to_string()),
        nonce: 1,
        public_key: None,
        signature: None,
    };

    let tx = Transaction::Transfer(transfer);

    // Verify transaction fields
    if let Transaction::Transfer(t) = &tx {
        assert_eq!(t.amount, Coord::from_num(100));
        assert_eq!(t.fee_area, Coord::from_num(5));
        assert_eq!(t.memo, Some("Test transfer".to_string()));
        assert_eq!(t.sender, alice_addr);
        assert_eq!(t.new_owner, bob_addr);
    } else {
        panic!("Expected Transfer transaction");
    }

    // Verify transaction hash
    let hash = tx.hash();
    assert_eq!(hash.len(), 32);

    Ok(())
}

#[test]
fn test_multiple_wallets_isolation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = get_test_dir()?;

    let wallets = vec!["wallet1", "wallet2", "wallet3"];
    let mut saved_wallets = Vec::new();

    // Create and save multiple wallets
    for name in &wallets {
        let wallet = create_test_wallet(name)?;
        let path = temp_dir.path().join(format!("{}.json", name));
        wallet.save(&path)?;
        saved_wallets.push((name.to_string(), path));
    }

    // Load and verify each wallet
    for (name, path) in saved_wallets {
        let loaded = Wallet::load(&path)?;
        assert_eq!(loaded.name, Some(name.clone()));
        assert!(loaded.address.len() == 64);
    }

    Ok(())
}

#[test]
fn test_wallet_secret_key_encoding() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = create_test_wallet("secret_test")?;

    // Secret key should be valid hex
    let decoded = hex::decode(&wallet.secret_key_hex);
    assert!(decoded.is_ok());

    // Decoded secret key should be 32 bytes (256-bit private key)
    let secret_bytes = decoded?;
    assert_eq!(secret_bytes.len(), 32);

    Ok(())
}

#[test]
fn test_transaction_fee_calculation() -> Result<(), Box<dyn std::error::Error>> {
    let alice = create_test_wallet("sender")?;
    let bob = create_test_wallet("recipient")?;

    let mut alice_addr = [0u8; 32];
    let mut bob_addr = [0u8; 32];
    hex::decode_to_slice(&alice.address, &mut alice_addr)?;
    hex::decode_to_slice(&bob.address, &mut bob_addr)?;

    let fee_area = Coord::from_num(10);

    let transfer = TransferTx {
        input_hash: [0u8; 32],
        new_owner: bob_addr,
        sender: alice_addr,
        amount: Coord::from_num(500),
        fee_area,
        memo: None,
        nonce: 1,
        public_key: None,
        signature: None,
    };

    let tx = Transaction::Transfer(transfer);

    // Verify fee calculation
    assert_eq!(tx.fee_area(), fee_area);
    assert_eq!(tx.fee_area(), Coord::from_num(10));

    Ok(())
}

#[test]
fn test_alice_to_bob_transaction() -> Result<(), Box<dyn std::error::Error>> {
    // Create two wallets: Alice (sender) and Bob (recipient)
    let alice = create_test_wallet("alice")?;
    let bob = create_test_wallet("bob")?;

    // Parse addresses
    let mut alice_addr = [0u8; 32];
    let mut bob_addr = [0u8; 32];
    hex::decode_to_slice(&alice.address, &mut alice_addr)?;
    hex::decode_to_slice(&bob.address, &mut bob_addr)?;

    // Create transfer: Alice sends 50 TRC to Bob with 2.5 fee
    let transfer = TransferTx {
        input_hash: [0u8; 32],
        new_owner: bob_addr,
        sender: alice_addr,
        amount: Coord::from_num(50),
        fee_area: Coord::from_num(2.5),
        memo: Some("Payment from Alice to Bob".to_string()),
        nonce: 1,
        public_key: None,
        signature: None,
    };

    let tx = Transaction::Transfer(transfer);

    // Verify transaction details
    if let Transaction::Transfer(t) = &tx {
        assert_eq!(t.sender, alice_addr);
        assert_eq!(t.new_owner, bob_addr);
        assert_eq!(t.amount, Coord::from_num(50));
        assert_eq!(t.memo, Some("Payment from Alice to Bob".to_string()));
    }

    // Verify both wallets are valid
    assert_eq!(alice.name, Some("alice".to_string()));
    assert_eq!(bob.name, Some("bob".to_string()));
    assert_ne!(alice.address, bob.address);

    Ok(())
}
