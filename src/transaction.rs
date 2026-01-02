//! Transaction module split into types and validation for better modularity

pub mod types;
pub mod validation;

pub use types::*;
// validation module kept internal; only types are re-exported publicly

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::TriangleState;
    use crate::crypto::Address;
    use crate::crypto::KeyPair;
    use crate::error::ChainError;
    use crate::geometry::{Coord, Point, Triangle};

    fn create_test_address(s: &str) -> Address {
        let mut address = [0u8; 32];
        let bytes = s.as_bytes();
        address[..bytes.len()].copy_from_slice(bytes);
        address
    }

    #[test]
    fn test_tx_validation_success() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            address,
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();

        let mut tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);
        assert!(tx.validate(&state).is_ok());
    }

    #[test]
    fn test_unsigned_transaction_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let address = create_test_address("test_address");

        let tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_invalid_signature_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let fake_signature = vec![0u8; 64];
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(fake_signature, public_key);

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_area_conservation_failure() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent);

        let bad_child = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(2.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(1.732)),
            None,
            create_test_address("test_owner"),
        );
        let children = vec![bad_child.clone(), bad_child.clone(), bad_child];

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let tx = SubdivisionTx::new(parent_hash, children, address, Coord::from_num(0), 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_double_spend_check() {
        let state = TriangleState::new();

        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        let children = parent.subdivide();

        let address = create_test_address("test_address");
        let tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_geometric_fee_deduction() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        let large_triangle = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(4.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.0), Coord::from_num(5.0)),
            None,
            sender_address,
        );

        let triangle_hash = large_triangle.hash();
        assert_eq!(large_triangle.area(), Coord::from_num(10.0));

        state.utxo_set.insert(triangle_hash, large_triangle);

        let fee_area = Coord::from_num(0.0001);
        let recipient_address = create_test_address("recipient_address");

        let mut tx = TransferTx::new(
            triangle_hash,
            recipient_address,
            sender_address,
            Coord::from_num(1.0), // Amount > fee
            fee_area,
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        assert!(tx.validate_with_state(&state).is_ok());

        let old_triangle = state.utxo_set.remove(&triangle_hash).unwrap();
        let new_value = old_triangle.effective_value() - fee_area;

        let new_triangle = Triangle::new_with_value(
            old_triangle.a,
            old_triangle.b,
            old_triangle.c,
            old_triangle.parent_hash,
            recipient_address,
            new_value,
        );

        let new_hash = new_triangle.hash();
        state.utxo_set.insert(new_hash, new_triangle);

        let result_triangle = state.utxo_set.get(&new_hash).unwrap();
        assert_eq!(result_triangle.owner, recipient_address);

        let expected_value = Coord::from_num(10.0) - Coord::from_num(0.0001);
        assert_eq!(result_triangle.effective_value(), expected_value);
        assert_eq!(result_triangle.area(), Coord::from_num(10.0));
    }

    #[test]
    fn test_geometric_fee_insufficient_value() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        let small_triangle = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(1.0)),
            None,
            sender_address,
        );

        let triangle_hash = small_triangle.hash();
        let triangle_area = small_triangle.area();
        state.utxo_set.insert(triangle_hash, small_triangle);

        let fee_area = triangle_area + Coord::from_num(0.1);

        let mut tx = TransferTx::new(
            triangle_hash,
            create_test_address("recipient"),
            sender_address,
            Coord::from_num(0),
            fee_area,
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        let result = tx.validate_with_state(&state);
        assert!(result.is_err());

        if let Err(ChainError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Insufficient"));
        } else {
            panic!("Expected InvalidTransaction error");
        }
    }

    #[test]
    fn test_negative_fee_rejected() {
        let keypair = KeyPair::generate().unwrap();

        let mut tx = TransferTx::new(
            [0u8; 32],
            create_test_address("recipient"),
            keypair.address(),
            Coord::from_num(0),
            Coord::from_num(-1.0), // Negative fee
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        let result = tx.validate();
        assert!(result.is_err());
    }
}
