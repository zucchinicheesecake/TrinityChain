//! Integration tests for TrinityChain API endpoints
//!
//! These tests verify that all dashboard endpoints respond correctly
//! with expected JSON structures after node startup and state changes.

use axum_test::TestServer;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use trinitychain::api::{build_api_router, Node};
use trinitychain::blockchain::Blockchain;
use trinitychain::network::NetworkNode;

#[tokio::test]
async fn test_dashboard_endpoints() {
    // Initialize minimal blockchain and network for testing
    let blockchain = Blockchain::new([0; 32], 1).expect("Failed to create blockchain");
    let blockchain = Arc::new(RwLock::new(blockchain));
    let network = Arc::new(NetworkNode::new(blockchain.clone()));

    // Create dummy orchestrator state for testing
    let state = Arc::new(RwLock::new(trinitychain::node::NodeState::Ready));

    // Create API node with shared state
    let api_node = Arc::new(Node::new_shared(blockchain, network, Some(state)));

    // Build the API router
    let app = build_api_router(api_node);

    // Create test server
    let server = TestServer::new(app).expect("Failed to create test server");

    // Test /api/health
    let response = server.get("/api/health").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert_eq!(json["status"], "healthy");
    assert!(json["timestamp"].is_string());

    // Test /api/blockchain/height
    let response = server.get("/api/blockchain/height").await;
    assert_eq!(response.status_code(), 200);
    let height: u64 = response.json();
    assert_eq!(height, 1); // Genesis block

    // Test /api/blockchain/stats
    let response = server.get("/api/blockchain/stats").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["height"].is_number());
    assert!(json["difficulty"].is_number());
    assert!(json["mempool_size"].is_number());
    assert!(json["total_blocks"].is_number());

    // Test /api/mining/status
    let response = server.get("/api/mining/status").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["is_mining"].is_boolean());
    assert!(json["blocks_mined"].is_number());

    // Test /api/network/peers
    let response = server.get("/api/network/peers").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["count"].is_number());
    assert!(json["peers"].is_array());

    // Test /api/mempool
    let response = server.get("/api/mempool").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["count"].is_number());
    assert!(json["transactions"].is_array());

    // Test /api/stats
    let response = server.get("/api/stats").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["total_requests"].is_number());
    assert!(json["successful_requests"].is_number());
    assert!(json["failed_requests"].is_number());
    assert!(json["mining_starts"].is_number());
    assert!(json["mining_stops"].is_number());
    assert!(json["transactions_submitted"].is_number());
    assert!(json["uptime_seconds"].is_number());
    assert!(json["blocks_mined"].is_number());
    assert!(json["is_mining"].is_boolean());

    // Test /api/blockchain/blocks (pagination)
    let response = server.get("/api/blockchain/blocks").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["blocks"].is_array());
    assert!(json["total"].is_number());
    assert!(json["page"].is_number());
    assert!(json["limit"].is_number());

    // Test /api/blockchain/block/0 (genesis block)
    let response = server.get("/api/blockchain/block/0").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["header"].is_object());
    assert!(json["transactions"].is_array());

    // Test invalid block height
    let response = server.get("/api/blockchain/block/999").await;
    assert_eq!(response.status_code(), 404);
    let json: Value = response.json();
    assert!(json["error"].is_string());

    // Test /api/wallet/create
    let response = server.post("/api/wallet/create").await;
    assert_eq!(response.status_code(), 200);
    let json: Value = response.json();
    assert!(json["address"].is_string());
    assert!(json["public_key"].is_string());
    assert!(json["private_key"].is_string());

    println!("✅ All dashboard endpoints responded correctly with expected JSON");
}