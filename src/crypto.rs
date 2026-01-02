//! Cryptographic primitives for TrinityChain

use crate::error::ChainError;
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use secp256k1::{
    constants::{COMPACT_SIGNATURE_SIZE, PUBLIC_KEY_SIZE, SECRET_KEY_SIZE},
    ecdsa::Signature,
    All, Message, PublicKey, Secp256k1, SecretKey,
};
use sha2::{Digest, Sha256};

/// A thread-safe, lazily initialized Secp256k1 context.
/// This prevents repeated, unnecessary context creation.
static SECP256K1_CONTEXT: Lazy<Secp256k1<All>> = Lazy::new(Secp256k1::new);

/// Type alias for the derived address, which is a 32-byte hash.
/// We use a fixed-size array for internal type safety and performance.
pub type Address = [u8; 32];

/// Convenience function to create an address from a string (hashes the string).
/// Useful for testing and debugging.
pub fn address_from_string(s: &str) -> Address {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hasher.finalize().into()
}

/// Convert an address to a hex string for display.
pub fn address_to_hex(addr: &Address) -> String {
    hex::encode(addr)
}

/// Convert a hex string to an address.
pub fn address_from_hex(hex_str: &str) -> Result<Address, ChainError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| ChainError::CryptoError(format!("Invalid hex address: {}", e)))?;
    if bytes.len() != 32 {
        return Err(ChainError::CryptoError(format!(
            "Address must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    bytes
        .try_into()
        .map_err(|_| ChainError::CryptoError("Failed to convert bytes into address".to_string()))
}

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl KeyPair {
    /// Generates a new random KeyPair using the OS random number generator.
    pub fn generate() -> Result<Self, ChainError> {
        let secret_key = SecretKey::new(&mut OsRng);
        // Using the context from the static Lazy
        let public_key = PublicKey::from_secret_key(&SECP256K1_CONTEXT, &secret_key);

        Ok(KeyPair {
            secret_key,
            public_key,
        })
    }

    /// Creates a KeyPair from an existing SecretKey.
    pub fn from_secret_key(secret_key: SecretKey) -> Self {
        // Using the context from the static Lazy
        let public_key = PublicKey::from_secret_key(&SECP256K1_CONTEXT, &secret_key);
        KeyPair {
            secret_key,
            public_key,
        }
    }

    /// Creates a KeyPair from raw secret key bytes.
    pub fn from_secret_bytes(bytes: &[u8]) -> Result<Self, ChainError> {
        // Use standard error message for length check
        let secret_key = SecretKey::from_slice(bytes).map_err(|e| {
            if bytes.len() != SECRET_KEY_SIZE {
                ChainError::CryptoError(format!(
                    "Secret key must be {} bytes, got {}",
                    SECRET_KEY_SIZE,
                    bytes.len()
                ))
            } else {
                ChainError::CryptoError(format!("Invalid secret key bytes: {}", e))
            }
        })?;

        Ok(Self::from_secret_key(secret_key))
    }

    /// Computes the blockchain address (SHA-256 hash of the compressed public key).
    pub fn address(&self) -> Address {
        // Use serialize() which returns a fixed-size array
        let pubkey_bytes: [u8; PUBLIC_KEY_SIZE] = self.public_key.serialize();
        Sha256::digest(pubkey_bytes).into()
    }

    /// Returns the KeyPair's public key as a compressed byte array.
    pub fn public_key_bytes(&self) -> [u8; PUBLIC_KEY_SIZE] {
        self.public_key.serialize()
    }

    /// Signs a message (which is first hashed using SHA-256) and returns the compact signature bytes.
    pub fn sign(&self, message: &[u8]) -> Result<[u8; COMPACT_SIGNATURE_SIZE], ChainError> {
        let digest = Sha256::digest(message);

        // Create message from digest; propagate any error
        let message = Message::from_digest_slice(&digest)
            .map_err(|e| ChainError::CryptoError(format!("Failed to create message: {}", e)))?;

        // Using the context from the static Lazy
        let signature = SECP256K1_CONTEXT.sign_ecdsa(&message, &self.secret_key);

        // Serialize directly into an array using to_compact() and array conversion (since secp256k1 v0.27)
        let compact_sig_bytes: [u8; COMPACT_SIGNATURE_SIZE] = signature.serialize_compact();
        Ok(compact_sig_bytes)
    }
}

/// Verifies an ECDSA signature given the raw public key bytes, message, and signature bytes.
pub fn verify_signature(
    public_key_bytes: &[u8],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<(), ChainError> {
    // Input validation: prefer using constant sizes in error messages for clarity
    if public_key_bytes.len() != PUBLIC_KEY_SIZE {
        return Err(ChainError::CryptoError(format!(
            "Public key must be exactly {} bytes (compressed), got {}",
            PUBLIC_KEY_SIZE,
            public_key_bytes.len()
        )));
    }
    if signature_bytes.len() != COMPACT_SIGNATURE_SIZE {
        return Err(ChainError::CryptoError(format!(
            "Signature must be exactly {} bytes (compact), got {}",
            COMPACT_SIGNATURE_SIZE,
            signature_bytes.len()
        )));
    }

    // Using the context from the static Lazy
    let public_key = PublicKey::from_slice(public_key_bytes)
        .map_err(|e| ChainError::CryptoError(format!("Invalid public key: {}", e)))?;

    // Hash the message
    let digest = Sha256::digest(message);

    let message = Message::from_digest_slice(&digest)
        .map_err(|e| ChainError::CryptoError(format!("Failed to create message: {}", e)))?;

    let signature = Signature::from_compact(signature_bytes)
        .map_err(|e| ChainError::CryptoError(format!("Invalid signature: {}", e)))?;

    // Return unit type on success, error on failure.
    SECP256K1_CONTEXT
        .verify_ecdsa(&message, &signature, &public_key)
        .map_err(|_| ChainError::CryptoError("Signature verification failed".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn test_key_generation() {
        let keypair = KeyPair::generate().unwrap();
        // Check compressed public key size
        assert_eq!(keypair.public_key_bytes().len(), PUBLIC_KEY_SIZE);
        // Check secret key size
        assert_eq!(keypair.secret_key.as_ref().len(), SECRET_KEY_SIZE);
    }

    #[test]
    fn test_address_generation() {
        let keypair = KeyPair::generate().unwrap();
        let address_bytes = keypair.address();
        // Address is a 32-byte SHA-256 hash
        assert_eq!(address_bytes.len(), 32);
        // Sanity check: verify hex encoding
        assert_eq!(hex::encode(address_bytes).len(), 64);
    }

    #[test]
    fn test_signing_and_verification() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Hello, TrinityChain!";

        let signature = keypair.sign(message).unwrap();
        let pubkey_bytes = keypair.public_key_bytes();

        let result = verify_signature(&pubkey_bytes, message, &signature);
        assert!(result.is_ok());
        // Check signature size
        assert_eq!(signature.len(), COMPACT_SIGNATURE_SIZE);
    }

    #[test]
    fn test_invalid_signature() {
        let keypair1 = KeyPair::generate().unwrap();
        let keypair2 = KeyPair::generate().unwrap();

        let message = b"Test message";
        let signature = keypair1.sign(message).unwrap();
        let pubkey2_bytes = keypair2.public_key_bytes();

        let result = verify_signature(&pubkey2_bytes, message, &signature);
        assert!(result.is_err());
        // Assert on the concrete error string for robust testing
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cryptographic error: Signature verification failed"
        );
    }

    #[test]
    fn test_tampered_message() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Original message";
        let tampered = b"Tampered message";

        let signature = keypair.sign(message).unwrap();
        let pubkey_bytes = keypair.public_key_bytes();

        let result = verify_signature(&pubkey_bytes, tampered, &signature);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cryptographic error: Signature verification failed"
        );
    }

    #[test]
    fn test_invalid_key_or_sig_length_check() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Test";
        let signature = keypair.sign(message).unwrap();
        let pubkey_bytes = keypair.public_key_bytes();

        // Invalid pubkey length
        let result = verify_signature(&pubkey_bytes[1..], message, &signature);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Public key must be exactly"));

        // Invalid signature length
        let result = verify_signature(&pubkey_bytes, message, &signature[1..]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signature must be exactly"));
    }

    #[test]
    fn test_from_secret_bytes_invalid_length() {
        let short_bytes = [0u8; SECRET_KEY_SIZE - 1];
        let result = KeyPair::from_secret_bytes(&short_bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Secret key must be"));
    }
}
