//! Error types for TrinityChain

use std::fmt;

#[derive(Debug, Clone)]
pub enum ChainError {
    InvalidBlockLinkage,
    NetworkError(String),
    DatabaseError(String),
    InvalidProofOfWork,
    InvalidMerkleRoot,
    InvalidTransaction(String),
    TriangleNotFound(String),
    CryptoError(String),
    WalletError(String),
    OrphanBlock,
    ApiError(String),
    AuthenticationError(String),
    MempoolFull,
    IoError(String),
    BincodeError(String),
    ForkNotFound,
    InvalidBlock(String),
    DoubleSpendDetected(String),
    BlockAlreadyExists,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChainError::InvalidBlockLinkage => write!(f, "Invalid block linkage"),
            ChainError::InvalidProofOfWork => write!(f, "Invalid proof of work"),
            ChainError::InvalidMerkleRoot => write!(f, "Invalid Merkle root"),
            ChainError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            ChainError::TriangleNotFound(msg) => write!(f, "Triangle not found: {}", msg),
            ChainError::CryptoError(msg) => write!(f, "Cryptographic error: {}", msg),
            ChainError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ChainError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ChainError::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            ChainError::OrphanBlock => write!(f, "Orphan block"),
            ChainError::ApiError(msg) => write!(f, "API error: {}", msg),
            ChainError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ChainError::MempoolFull => write!(f, "Mempool is full"),
            ChainError::IoError(msg) => write!(f, "IO error: {}", msg),
            ChainError::BincodeError(msg) => write!(f, "Bincode error: {}", msg),
            ChainError::ForkNotFound => write!(f, "Fork not found"),
            ChainError::InvalidBlock(msg) => write!(f, "Invalid block: {}", msg),
            ChainError::DoubleSpendDetected(msg) => write!(f, "Double spend detected: {}", msg),
            ChainError::BlockAlreadyExists => write!(f, "Block already exists"),
        }
    }
}

impl std::error::Error for ChainError {}

impl From<std::io::Error> for ChainError {
    fn from(err: std::io::Error) -> Self {
        ChainError::IoError(err.to_string())
    }
}

impl From<Box<bincode::ErrorKind>> for ChainError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        ChainError::BincodeError(err.to_string())
    }
}

/// Convenience alias used across the crate
pub type Result<T> = std::result::Result<T, ChainError>;
