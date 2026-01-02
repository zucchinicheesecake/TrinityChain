use crate::error::ChainError;
use crate::transaction::Transaction;

pub fn validate_no_double_spend(block: &crate::blockchain::core::chain::Block) -> Result<(), ChainError> {
    let mut seen_inputs = std::collections::HashMap::new();
    for tx in &block.transactions {
        let input_hash = match tx {
            Transaction::Transfer(t) => Some(t.input_hash),
            Transaction::Subdivision(s) => Some(s.parent_hash),
            _ => None,
        };

        if let Some(hash) = input_hash {
            if let Some(conflicting_tx_hash) = seen_inputs.get(&hash) {
                return Err(ChainError::InvalidTransaction(format!(
                    "Double spend detected in block. UTXO {} is spent by both {} and {}",
                    hex::encode(hash),
                    hex::encode(conflicting_tx_hash),
                    hex::encode(tx.hash())
                )));
            }
            seen_inputs.insert(hash, tx.hash());
        }
    }
    Ok(())
}
