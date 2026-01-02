/// Transaction types for TrinityChain
use crate::blockchain::Sha256Hash;
use crate::crypto::Address;
use crate::error::ChainError;
use crate::geometry::{Coord, Triangle};
use sha2::{Digest, Sha256};

/// Maximum transaction size in bytes (100KB) to prevent DoS
pub const MAX_TRANSACTION_SIZE: usize = 100_000;

/// A transaction that can occur in a block
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Transaction {
    Transfer(TransferTx),
    Subdivision(SubdivisionTx),
    Coinbase(CoinbaseTx),
}

impl Transaction {
    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    /// Validate transaction size to prevent DoS attacks
    pub fn validate_size(&self) -> Result<(), ChainError> {
        let serialized = bincode::serialize(self)
            .map_err(|e| ChainError::InvalidTransaction(format!("Serialization failed: {}", e)))?;

        if serialized.len() > MAX_TRANSACTION_SIZE {
            return Err(ChainError::InvalidTransaction(format!(
                "Transaction too large: {} bytes (max: {})",
                serialized.len(),
                MAX_TRANSACTION_SIZE
            )));
        }
        Ok(())
    }

    /// Get the geometric fee area for this transaction
    pub fn fee_area(&self) -> crate::geometry::Coord {
        match self {
            Transaction::Subdivision(tx) => tx.fee_area,
            Transaction::Transfer(tx) => tx.fee_area,
            Transaction::Coinbase(_) => Coord::from_num(0), // Coinbase has no fee
        }
    }

    /// Get the fee as u64 (for backward compatibility, converts fee_area)
    /// Deprecated: Use fee_area() for geometric fees
    pub fn fee(&self) -> u64 {
        self.fee_area().to_num::<u64>()
    }

    /// Calculate the hash of this transaction
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            Transaction::Subdivision(tx) => {
                hasher.update(tx.parent_hash);
                for child in &tx.children {
                    hasher.update(child.hash());
                }
                hasher.update(tx.owner_address);
                hasher.update(tx.fee_area.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
            Transaction::Coinbase(tx) => {
                hasher.update("coinbase".as_bytes());
                hasher.update(tx.reward_area.to_le_bytes());
                hasher.update(tx.beneficiary_address);
                hasher.update(tx.nonce.to_le_bytes());
            }
            Transaction::Transfer(tx) => {
                hasher.update("transfer".as_bytes());
                hasher.update(tx.input_hash);
                hasher.update(tx.new_owner);
                hasher.update(tx.sender);
                hasher.update(tx.amount.to_le_bytes());
                hasher.update(tx.fee_area.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
        };
        hasher.finalize().into()
    }
}

/// Subdivision transaction: splits one parent triangle into three children
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubdivisionTx {
    pub parent_hash: Sha256Hash,
    pub children: Vec<Triangle>,
    pub owner_address: Address,
    pub fee_area: Coord,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

impl SubdivisionTx {
    pub fn new(
        parent_hash: Sha256Hash,
        children: Vec<Triangle>,
        owner_address: Address,
        fee_area: Coord,
        nonce: u64,
    ) -> Self {
        SubdivisionTx {
            parent_hash,
            children,
            owner_address,
            fee_area,
            nonce,
            signature: None,
            public_key: None,
        }
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.parent_hash);
        for child in &self.children {
            message.extend_from_slice(&child.hash());
        }
        message.extend_from_slice(&self.owner_address);
        message.extend_from_slice(&self.fee_area.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }
}

/// Coinbase transaction: miner reward
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoinbaseTx {
    pub reward_area: Coord,
    pub beneficiary_address: Address,
    #[serde(default)]
    pub nonce: u64,
}

impl CoinbaseTx {
    /// Maximum reward area that can be claimed in a coinbase transaction
    pub const MAX_REWARD_AREA: Coord = Coord::from_bits(1000i64 << 32);

    pub fn validate(&self) -> Result<(), ChainError> {
        // Validate reward area is within acceptable bounds
        if self.reward_area <= Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Coinbase reward area must be greater than zero".to_string(),
            ));
        }

        if self.reward_area > Self::MAX_REWARD_AREA {
            return Err(ChainError::InvalidTransaction(format!(
                "Coinbase reward area {} exceeds maximum {}",
                self.reward_area,
                Self::MAX_REWARD_AREA
            )));
        }

        // Validate beneficiary address is not empty
        if self.beneficiary_address == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "Coinbase beneficiary address cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Transfer transaction - moves ownership of a triangle
/// Fee is now geometric: fee_area is deducted from the triangle's value
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferTx {
    pub input_hash: Sha256Hash,
    pub new_owner: Address,
    pub sender: Address,
    /// Amount being sent to the new owner
    pub amount: crate::geometry::Coord,
    /// Geometric fee: area deducted from triangle value and given to miner
    pub fee_area: crate::geometry::Coord,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
    #[serde(default)]
    pub memo: Option<String>,
}

impl TransferTx {
    /// Maximum memo length (256 characters)
    pub const MAX_MEMO_LENGTH: usize = 256;
    pub fn new(
        input_hash: Sha256Hash,
        new_owner: Address,
        sender: Address,
        amount: crate::geometry::Coord,
        fee_area: crate::geometry::Coord,
        nonce: u64,
    ) -> Self {
        TransferTx {
            input_hash,
            new_owner,
            sender,
            amount,
            fee_area,
            nonce,
            signature: None,
            public_key: None,
            memo: None,
        }
    }

    pub fn with_memo(mut self, memo: String) -> Result<Self, ChainError> {
        if memo.len() > Self::MAX_MEMO_LENGTH {
            return Err(ChainError::InvalidTransaction(format!(
                "Memo exceeds maximum length of {} characters",
                Self::MAX_MEMO_LENGTH
            )));
        }
        self.memo = Some(memo);
        Ok(self)
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice("TRANSFER:".as_bytes());
        message.extend_from_slice(&self.input_hash);
        message.extend_from_slice(&self.new_owner);
        message.extend_from_slice(&self.sender);
        message.extend_from_slice(&self.amount.to_le_bytes());
        // Use f64 bytes for geometric fee
        message.extend_from_slice(&self.fee_area.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }
}
