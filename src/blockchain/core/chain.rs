use crate::crypto::Address;
use crate::error::ChainError;
use crate::mempool::Mempool;
use crate::miner::mine_block;
use crate::transaction::{CoinbaseTx, Transaction};
use crate::persistence::{Persistence, InMemoryPersistence};
use crate::geometry::Coord;
use sha2::{Digest, Sha256};
// HashMap not needed in this module currently

pub type Sha256Hash = [u8; 32];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub previous_hash: Sha256Hash,
    pub merkle_root: Sha256Hash,
    pub difficulty: u32,
    pub nonce: u64,
}

impl BlockHeader {
    pub fn hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.merkle_root);
        hasher.update(self.difficulty.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        height: u64,
        previous_hash: Sha256Hash,
        difficulty: u32,
        transactions: Vec<Transaction>,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let merkle_root = Block::calculate_merkle_root(&transactions);

        Block {
            header: BlockHeader {
                height,
                timestamp,
                previous_hash,
                merkle_root,
                difficulty,
                nonce: 0,
            },
            transactions,
        }
    }

    pub fn hash(&self) -> Sha256Hash {
        self.header.hash()
    }

    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Sha256Hash {
        let mut hasher = Sha256::new();
        for tx in transactions {
            hasher.update(tx.hash());
        }
        hasher.finalize().into()
    }

    pub fn hash_as_u256(hash: &Sha256Hash) -> [u8; 32] {
        *hash
    }

    pub fn hash_to_target(difficulty: &u32) -> [u8; 32] {
        let mut target = [0xFF; 32];
        let leading_zeros = *difficulty / 8;
        let partial_bits = *difficulty % 8;

        for item in target.iter_mut().take(leading_zeros as usize) {
            *item = 0;
        }

        if leading_zeros < 32 && partial_bits > 0 {
            target[leading_zeros as usize] = (0xFF >> partial_bits) as u8;
        }
        target
    }
}

// Blockchain struct and implementation
use crate::blockchain::core::state::TriangleState;
use crate::blockchain::core::validation::validate_no_double_spend;
// These imports were not used after refactor; keep commented for future use if needed.
// use crate::transaction::TransferTx;
// use crate::geometry::GEOMETRIC_TOLERANCE;

pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;
pub const TARGET_BLOCK_TIME: u64 = 30;

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub difficulty: u32,
    pub mempool: Mempool,
    pub state: TriangleState,
    pub persistence: Box<dyn Persistence>,
}

impl Clone for Blockchain {
    fn clone(&self) -> Self {
        Self {
            blocks: self.blocks.clone(),
            difficulty: self.difficulty,
            mempool: self.mempool.clone(),
            state: self.state.clone(),
            // Persistence cannot be cloned as a trait object; use a fresh in-memory backend for clones.
            persistence: Box::new(InMemoryPersistence::new()),
        }
    }
}

impl Blockchain {
    /// Create a new `Blockchain` using an in-memory persistence backend.
    pub fn new(genesis_miner_address: Address, initial_difficulty: u32) -> Result<Self, ChainError> {
        Self::new_with_persistence(genesis_miner_address, initial_difficulty, Box::new(InMemoryPersistence::new()))
    }

    /// Create a new `Blockchain` with the provided persistence backend.
    pub fn new_with_persistence(
        genesis_miner_address: Address,
        initial_difficulty: u32,
        persistence: Box<dyn Persistence>,
    ) -> Result<Self, ChainError> {
        let genesis_block = Self::create_genesis_block(genesis_miner_address, initial_difficulty)?;

        let mut blockchain = Blockchain {
            blocks: vec![],
            difficulty: initial_difficulty,
            mempool: Mempool::new(),
            state: TriangleState::new(),
            persistence,
        };

        blockchain.apply_block(genesis_block)?;
        Ok(blockchain)
    }

    fn create_genesis_block(miner_address: Address, initial_difficulty: u32) -> Result<Block, ChainError> {
        let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(1_000_000.0),
            beneficiary_address: miner_address,
            nonce: 0,
        });

        let transactions = vec![coinbase_tx];
        let merkle_root = Block::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            height: 0,
            timestamp: 1672531200000,
            previous_hash: [0u8; 32],
            merkle_root,
            difficulty: initial_difficulty,
            nonce: 0,
        };

        let genesis_block = Block { header, transactions };
        mine_block(genesis_block)
    }

    pub fn calculate_block_reward(height: u64) -> f64 {
        const INITIAL_REWARD: f64 = 50.0;
        const HALVING_INTERVAL: u64 = 210000;

        let halving_count = height / HALVING_INTERVAL;
        if halving_count >= 64 {
            0.0
        } else {
            INITIAL_REWARD / (2u64.pow(halving_count as u32) as f64)
        }
    }

    pub fn apply_block(&mut self, block: Block) -> Result<(), ChainError> {
        let is_genesis = block.header.height == 0;

        if !is_genesis {
            let last_block = self.blocks.last().ok_or_else(|| {
                ChainError::InvalidBlock("Cannot apply non-genesis block; the chain is empty.".to_string())
            })?;

            if block.header.height != last_block.header.height + 1 {
                return Err(ChainError::InvalidBlock(format!(
                    "Invalid block height. Expected {}, but got {}.",
                    last_block.header.height + 1,
                    block.header.height
                )));
            }

            if block.header.previous_hash != last_block.hash() {
                return Err(ChainError::InvalidBlock(format!(
                    "Invalid previous block hash. Expected {}, but got {}.",
                    hex::encode(last_block.hash()),
                    hex::encode(block.header.previous_hash)
                )));
            }
        } else if !self.blocks.is_empty() {
            return Err(ChainError::InvalidBlock("Genesis block can only be applied to an empty chain.".to_string()));
        }

        if !self.verify_pow(&block) {
            return Err(ChainError::InvalidBlock("Invalid Proof-of-Work: Block hash does not meet difficulty target.".to_string()));
        }

        let mut temp_state = self.state.clone();

        validate_no_double_spend(&block)?;

        for (i, tx) in block.transactions.iter().enumerate() {
            tx.validate_size()?;
            if i == 0 {
                if !matches!(tx, Transaction::Coinbase(_)) {
                    return Err(ChainError::InvalidBlock("First transaction in a block must be a Coinbase transaction.".to_string()));
                }
            } else {
                tx.validate(&temp_state)?;
            }
            temp_state.apply_transaction(tx, block.header.height)?;
        }

        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);
        if expected_merkle_root != block.header.merkle_root {
            return Err(ChainError::InvalidBlock(format!(
                "Merkle root mismatch. Expected {}, but got {}.",
                hex::encode(expected_merkle_root),
                hex::encode(block.header.merkle_root)
            )));
        }

        self.blocks.push(block.clone());
        self.state = temp_state;

        for tx in &block.transactions {
            self.mempool.remove_transaction(&tx.hash());
        }

        // Persist blockchain state after successfully applying the block.
        let _ = self.persistence.save_blockchain_state(&block, &self.state, self.difficulty as u64);

        self.adjust_difficulty();

        Ok(())
    }

    fn adjust_difficulty(&mut self) {
        let current_height = self.blocks.last().map_or(0, |b| b.header.height);
        if current_height > 0 && current_height.is_multiple_of(DIFFICULTY_ADJUSTMENT_INTERVAL) {
            let last_adjustment_block = self.blocks.get((current_height - DIFFICULTY_ADJUSTMENT_INTERVAL) as usize);
            if let Some(last_adjustment_block) = last_adjustment_block {
                let last_block = self.blocks.last().unwrap();
                let actual_time = last_block.header.timestamp - last_adjustment_block.header.timestamp;
                let expected_time = (DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME) * 1000;
                let ratio = actual_time as f64 / expected_time as f64;
                let ratio = ratio.clamp(0.25, 4.0);
                let new_difficulty = (self.difficulty as f64 * ratio) as u32;
                self.difficulty = new_difficulty.max(1);
            }
        }
    }

    fn verify_pow(&self, block: &Block) -> bool {
        let hash_target = Block::hash_to_target(&block.header.difficulty);
        let block_hash_int = Block::hash_as_u256(&block.hash());
        block_hash_int <= hash_target
    }
}
