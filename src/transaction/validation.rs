/// Validation logic for transactions separated from type definitions
use crate::error::ChainError;
use crate::geometry::GEOMETRIC_TOLERANCE;
use crate::transaction::types::{SubdivisionTx, Transaction, TransferTx};

use crate::blockchain::TriangleState;

impl Transaction {
    /// Validate this transaction against the current UTXO state
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        match self {
            Transaction::Subdivision(tx) => tx.validate(state),
            Transaction::Coinbase(tx) => tx.validate(),
            Transaction::Transfer(tx) => tx.validate(),
        }
    }
}

impl SubdivisionTx {
    /// Validates just the signature of the transaction, without access to blockchain state.
    /// This is useful for early validation in the mempool.
    pub fn validate_signature(&self) -> Result<(), ChainError> {
        let (signature, public_key) = match (&self.signature, &self.public_key) {
            (Some(sig), Some(pk)) => (sig, pk),
            _ => {
                return Err(ChainError::InvalidTransaction(
                    "Transaction not signed".to_string(),
                ))
            }
        };

        let message = self.signable_message();
        crate::crypto::verify_signature(public_key, &message, signature)?;

        Ok(())
    }

    /// Performs a full validation of the transaction against the current blockchain state.
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First, perform a stateless signature check.
        self.validate_signature()?;

        // Then, validate against the current state (UTXO set).
        let parent = match state.utxo_set.get(&self.parent_hash) {
            Some(triangle) => triangle,
            None => {
                return Err(ChainError::TriangleNotFound(format!(
                    "Parent triangle {} not found in UTXO set",
                    hex::encode(self.parent_hash)
                )))
            }
        };

        // Verify that the transaction's owner address matches the parent triangle's owner
        if parent.owner != self.owner_address {
            return Err(ChainError::InvalidTransaction(format!(
                "Subdivision transaction owner {} does not match parent triangle owner {}",
                hex::encode(self.owner_address),
                hex::encode(parent.owner)
            )));
        }

        let expected_children = parent.subdivide();

        if self.children.len() != 3 {
            return Err(ChainError::InvalidTransaction(
                "Subdivision must produce exactly 3 children".to_string(),
            ));
        }

        for (i, child) in self.children.iter().enumerate() {
            let expected = &expected_children[i];
            if !child.a.equals(&expected.a)
                || !child.b.equals(&expected.b)
                || !child.c.equals(&expected.c)
            {
                return Err(ChainError::InvalidTransaction(format!(
                    "Child {} geometry does not match expected subdivision",
                    i
                )));
            }
        }

        Ok(())
    }
}

impl TransferTx {
    /// Stateless validation: checks signature, addresses, memo, and fee bounds.
    /// Does NOT validate against UTXO state - use validate_with_state() for that.
    pub fn validate(&self) -> Result<(), ChainError> {
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction(
                "Transfer not signed".to_string(),
            ));
        }

        // Validate addresses are not empty
        if self.sender == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "Sender address cannot be empty".to_string(),
            ));
        }
        if self.new_owner == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "New owner address cannot be empty".to_string(),
            ));
        }
        // Prevent self-sends
        if self.sender == self.new_owner {
            return Err(ChainError::InvalidTransaction(
                "Sender and new owner cannot be the same".to_string(),
            ));
        }

        // Validate amount and fee are non-negative and not both zero
        if self.amount < crate::geometry::Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Transfer amount cannot be negative".to_string(),
            ));
        }
        if self.fee_area < crate::geometry::Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Fee area cannot be negative".to_string(),
            ));
        }
        if self.amount == crate::geometry::Coord::from_num(0)
            && self.fee_area == crate::geometry::Coord::from_num(0)
        {
            return Err(ChainError::InvalidTransaction(
                "Amount and fee cannot both be zero".to_string(),
            ));
        }

        // Validate memo length to prevent DoS attacks
        if let Some(ref memo) = self.memo {
            if memo.len() > Self::MAX_MEMO_LENGTH {
                return Err(ChainError::InvalidTransaction(format!(
                    "Memo exceeds maximum length of {} characters",
                    Self::MAX_MEMO_LENGTH
                )));
            }
        }

        let (signature, public_key) = match (&self.signature, &self.public_key) {
            (Some(sig), Some(pk)) => (sig, pk),
            _ => {
                return Err(ChainError::InvalidTransaction(
                    "Transfer not signed".to_string(),
                ))
            }
        };

        let message = self.signable_message();
        crate::crypto::verify_signature(public_key, &message, signature)?;

        Ok(())
    }

    /// Full validation including UTXO state check.
    /// Ensures: input triangle exists AND input.effective_value() > fee_area + TOLERANCE
    pub fn validate_with_state(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First perform stateless validation
        self.validate()?;

        // Check input triangle exists in UTXO set
        let input_triangle = state.utxo_set.get(&self.input_hash).ok_or_else(|| {
            ChainError::TriangleNotFound(format!(
                "Transfer input {} not found in UTXO set",
                hex::encode(self.input_hash)
            ))
        })?;

        // Area balance check: input value must be strictly greater than fee
        let input_value = input_triangle.effective_value();
        let total_spent = self.amount + self.fee_area;
        let remaining_value = input_value - total_spent;

        if remaining_value < GEOMETRIC_TOLERANCE {
            return Err(ChainError::InvalidTransaction(format!(
                "Insufficient triangle value: input has {} but amount + fee_area is {}, leaving {} (minimum: {})",
                input_value, total_spent, remaining_value, GEOMETRIC_TOLERANCE
            )));
        }

        // Verify sender owns the triangle
        if input_triangle.owner != self.sender {
            return Err(ChainError::InvalidTransaction(format!(
                "Sender {} does not own input triangle (owned by {})",
                hex::encode(self.sender),
                hex::encode(input_triangle.owner)
            )));
        }

        Ok(())
    }
}
