use crate::crypto::Address;
use crate::error::ChainError;
use crate::geometry::{Coord, Point, Triangle, GEOMETRIC_TOLERANCE};
use crate::transaction::{Transaction, TransferTx};
use std::collections::HashMap;

use super::chain::Sha256Hash;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TriangleState {
    pub utxo_set: HashMap<Sha256Hash, Triangle>,
    pub address_balances: HashMap<Address, Coord>,
}

impl TriangleState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild_address_balances(&mut self) {
        self.address_balances.clear();
        for triangle in self.utxo_set.values() {
            *self.address_balances.entry(triangle.owner).or_insert(Coord::from_num(0)) += triangle.effective_value();
        }
    }

    pub fn get_balance(&self, address: &Address) -> Coord {
        *self.address_balances.get(address).unwrap_or(&Coord::from_num(0))
    }

    pub fn apply_transaction(&mut self, tx: &Transaction, _block_height: u64) -> Result<(), ChainError> {
        match tx {
            Transaction::Coinbase(tx) => {
                let new_triangle = Triangle::new(
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    None,
                    tx.beneficiary_address,
                ).with_effective_value(tx.reward_area);

                let tx_hash = Transaction::Coinbase(tx.clone()).hash();
                self.utxo_set.insert(tx_hash, new_triangle);
                *self.address_balances.entry(tx.beneficiary_address).or_insert(Coord::from_num(0)) += tx.reward_area;
            }
            Transaction::Transfer(tx) => {
                let input_hash = tx.input_hash;
                let consumed_triangle = self.utxo_set.remove(&input_hash).ok_or_else(|| ChainError::TriangleNotFound(format!("Input UTXO not found for transfer: {}", hex::encode(input_hash))))?;

                if consumed_triangle.owner != tx.sender {
                    self.utxo_set.insert(input_hash, consumed_triangle.clone());
                    return Err(ChainError::InvalidTransaction(format!("Sender {} does not own input UTXO (owned by {})", hex::encode(tx.sender), hex::encode(consumed_triangle.owner))));
                }

                let input_value = consumed_triangle.effective_value();
                let total_spent = tx.amount + tx.fee_area;
                let remaining_value = input_value - total_spent;

                let sender_balance = self.address_balances.entry(tx.sender).or_insert(Coord::from_num(0));
                *sender_balance -= input_value;
                if *sender_balance < Coord::from_num(0) { *sender_balance = Coord::from_num(0); }

                let new_owner_triangle = consumed_triangle.clone().change_owner(tx.new_owner).with_effective_value(tx.amount);
                let tx_hash = Transaction::Transfer(tx.clone()).hash();
                self.utxo_set.insert(tx_hash, new_owner_triangle);
                *self.address_balances.entry(tx.new_owner).or_insert(Coord::from_num(0)) += tx.amount;

                if remaining_value > GEOMETRIC_TOLERANCE {
                    let change_tx = Transaction::Transfer(TransferTx {
                        input_hash: tx_hash,
                        new_owner: tx.sender,
                        sender: tx.sender,
                        amount: remaining_value,
                        fee_area: Coord::from_num(0),
                        nonce: tx.nonce + 1,
                        signature: None,
                        public_key: None,
                        memo: Some("Change".to_string()),
                    });

                    let change_hash = change_tx.hash();
                    let change_triangle = consumed_triangle.change_owner(tx.sender).with_effective_value(remaining_value);
                    self.utxo_set.insert(change_hash, change_triangle);
                    *self.address_balances.entry(tx.sender).or_insert(Coord::from_num(0)) += remaining_value;
                }
            }
            Transaction::Subdivision(tx) => {
                let input_hash = tx.parent_hash;
                let consumed_triangle = self.utxo_set.remove(&input_hash).ok_or_else(|| ChainError::TriangleNotFound(format!("Parent UTXO for subdivision not found: {}", hex::encode(input_hash))))?;

                if consumed_triangle.owner != tx.owner_address {
                    self.utxo_set.insert(input_hash, consumed_triangle.clone());
                    return Err(ChainError::InvalidTransaction(format!("Subdivision owner {} does not match parent triangle owner {}", hex::encode(tx.owner_address), hex::encode(consumed_triangle.owner))));
                }

                let parent_value = consumed_triangle.effective_value();
                let owner_balance = self.address_balances.entry(tx.owner_address).or_insert(Coord::from_num(0));
                *owner_balance -= parent_value;
                if *owner_balance < Coord::from_num(0) { *owner_balance = Coord::from_num(0); }

                let total_child_value: Coord = tx.children.iter().map(|c| c.effective_value()).sum();
                let expected_value = parent_value - tx.fee_area;

                if (total_child_value - expected_value).abs() > GEOMETRIC_TOLERANCE {
                    self.utxo_set.insert(input_hash, consumed_triangle);
                    *self.address_balances.entry(tx.owner_address).or_insert(Coord::from_num(0)) += parent_value;
                    return Err(ChainError::InvalidTransaction(format!("Value mismatch in subdivision: parent ({}) - fee ({}) != children total ({}).", parent_value, tx.fee_area, total_child_value)));
                }

                for child in &tx.children {
                    self.utxo_set.insert(child.hash(), child.clone());
                    *self.address_balances.entry(tx.owner_address).or_insert(Coord::from_num(0)) += child.effective_value();
                }
            }
        }
        Ok(())
    }
}
