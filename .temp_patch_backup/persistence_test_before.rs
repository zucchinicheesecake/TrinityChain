// backup of original test_save_and_load_blockchain
// (kept for reference)

#[test]
fn test_save_and_load_blockchain() {
    let db = Database::open(":memory:").unwrap();
    let chain = Blockchain::new(create_test_address("miner"), 1).unwrap();

    db.save_blockchain_state(&chain.blocks[0], &chain.state, chain.difficulty as u64)
        .unwrap();

    let loaded_chain = db.load_blockchain().unwrap();

    assert_eq!(loaded_chain.blocks.len(), 1);
    assert_eq!(loaded_chain.blocks[0].header.height, 0);
    assert_eq!(loaded_chain.difficulty, chain.difficulty);
}
