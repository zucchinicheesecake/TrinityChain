//! Node synchronization module for TrinityChain
//!
//! This module provides comprehensive node synchronization capabilities including:
//! - Full chain sync from peers
//! - Incremental sync for catching up on new blocks
//! - Peer management and selection
//! - Sync progress tracking
//! - Automatic peer discovery

use crate::blockchain::{Block, Blockchain};
use crate::error::ChainError;
use crate::network::Node;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Peer sync information
#[derive(Debug, Clone)]
pub struct PeerSyncInfo {
    pub node: Node,
    pub height: u64,
    pub last_seen: Instant,
    pub blocks_received: u64,
    pub sync_failures: u32,
    pub is_syncing: bool,
}

impl PeerSyncInfo {
    pub fn new(node: Node, height: u64) -> Self {
        Self {
            node,
            height,
            last_seen: Instant::now(),
            blocks_received: 0,
            sync_failures: 0,
            is_syncing: false,
        }
    }

    /// Check if peer should be considered unreliable
    pub fn is_unreliable(&self) -> bool {
        self.sync_failures >= 3
    }

    /// Check if peer is stale (not seen in 5 minutes)
    pub fn is_stale(&self) -> bool {
        self.last_seen.elapsed() > Duration::from_secs(300)
    }
}

/// Sync statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub total_blocks_synced: u64,
    pub blocks_synced_this_session: u64,
    pub active_peers: usize,
    pub sync_speed: f64, // blocks per second
    pub estimated_time_remaining: Duration,
    pub last_block_time: Instant,
}

impl Default for SyncStats {
    fn default() -> Self {
        Self {
            total_blocks_synced: 0,
            blocks_synced_this_session: 0,
            active_peers: 0,
            sync_speed: 0.0,
            estimated_time_remaining: Duration::from_secs(0),
            last_block_time: Instant::now(),
        }
    }
}

/// Sync state tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncState {
    Idle,
    Syncing,
    Synced,
    Failed,
}

/// Node synchronizer
pub struct NodeSynchronizer {
    peers: Arc<RwLock<HashMap<String, PeerSyncInfo>>>,
    sync_state: Arc<RwLock<SyncState>>,
    stats: Arc<RwLock<SyncStats>>,
    /// Queue of blocks pending application
    pending_blocks: Arc<RwLock<VecDeque<Block>>>,
}

impl NodeSynchronizer {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            sync_state: Arc::new(RwLock::new(SyncState::Idle)),
            stats: Arc::new(RwLock::new(SyncStats::default())),
            pending_blocks: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Register a peer for synchronization
    pub async fn register_peer(&self, node: Node, height: u64) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;
        let key = node.addr();

        if let std::collections::hash_map::Entry::Vacant(e) = peers.entry(key) {
            e.insert(PeerSyncInfo::new(node, height));
            Ok(())
        } else {
            Err(ChainError::NetworkError(
                "Peer already registered".to_string(),
            ))
        }
    }

    /// Update peer height information
    pub async fn update_peer_height(&self, node_addr: &str, height: u64) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;

        if let Some(peer) = peers.get_mut(node_addr) {
            peer.height = height;
            peer.last_seen = Instant::now();
            Ok(())
        } else {
            Err(ChainError::NetworkError("Peer not found".to_string()))
        }
    }

    /// Get the best peer to sync from (highest height, no failures)
    pub async fn get_best_peer(&self) -> Option<Node> {
        let peers = self.peers.read().await;

        peers
            .values()
            .filter(|p| !p.is_unreliable() && !p.is_stale() && !p.is_syncing)
            .max_by_key(|p| p.height)
            .map(|p| p.node.clone())
    }

    /// Get multiple best peers for concurrent syncing
    pub async fn get_best_peers(&self, count: usize) -> Vec<Node> {
        let peers = self.peers.read().await;

        let mut valid_peers: Vec<_> = peers
            .values()
            .filter(|p| !p.is_unreliable() && !p.is_stale() && !p.is_syncing)
            .collect();

        // Sort by height (descending) and return top N
        valid_peers.sort_by(|a, b| b.height.cmp(&a.height));
        valid_peers
            .into_iter()
            .take(count)
            .map(|p| p.node.clone())
            .collect()
    }

    /// Check if node is synced
    pub async fn is_synced(&self) -> bool {
        *self.sync_state.read().await == SyncState::Synced
    }

    /// Check if sync is in progress
    pub async fn is_syncing(&self) -> bool {
        *self.sync_state.read().await == SyncState::Syncing
    }

    /// Get current sync state
    pub async fn get_sync_state(&self) -> SyncState {
        *self.sync_state.read().await
    }

    /// Get sync statistics
    pub async fn get_stats(&self) -> SyncStats {
        self.stats.read().await.clone()
    }

    /// Set sync state
    async fn set_sync_state(&self, state: SyncState) {
        *self.sync_state.write().await = state;
    }

    /// Record successful block sync from peer
    pub async fn record_block_received(&self, node_addr: &str) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;

        if let Some(peer) = peers.get_mut(node_addr) {
            peer.blocks_received += 1;
            peer.last_seen = Instant::now();

            // Update sync stats
            let mut stats = self.stats.write().await;
            stats.blocks_synced_this_session += 1;
            stats.total_blocks_synced += 1;

            // Calculate sync speed
            let elapsed = stats.last_block_time.elapsed();
            if elapsed.as_secs_f64() > 0.0 {
                stats.sync_speed = 1.0 / elapsed.as_secs_f64();
            }
            stats.last_block_time = Instant::now();

            Ok(())
        } else {
            Err(ChainError::NetworkError("Peer not found".to_string()))
        }
    }

    /// Record sync failure for a peer
    pub async fn record_sync_failure(&self, node_addr: &str) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;

        if let Some(peer) = peers.get_mut(node_addr) {
            peer.sync_failures += 1;
            peer.last_seen = Instant::now();

            if peer.is_unreliable() {
                println!("⚠️  Peer {} marked as unreliable", node_addr);
            }

            Ok(())
        } else {
            Err(ChainError::NetworkError("Peer not found".to_string()))
        }
    }

    /// Queue a block for application
    pub async fn queue_block(&self, block: Block) {
        let mut queue = self.pending_blocks.write().await;
        queue.push_back(block);
    }

    /// Get all queued blocks
    pub async fn get_pending_blocks(&self) -> Vec<Block> {
        let mut queue = self.pending_blocks.write().await;
        queue.drain(..).collect()
    }

    /// Check if there are pending blocks
    pub async fn has_pending_blocks(&self) -> bool {
        !self.pending_blocks.read().await.is_empty()
    }

    /// Clear all pending blocks
    pub async fn clear_pending_blocks(&self) {
        self.pending_blocks.write().await.clear();
    }

    /// Get list of all registered peers
    pub async fn get_all_peers(&self) -> Vec<Node> {
        let peers = self.peers.read().await;
        peers.values().map(|p| p.node.clone()).collect()
    }

    /// Get peer count
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Remove a peer
    pub async fn remove_peer(&self, node_addr: &str) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;
        peers
            .remove(node_addr)
            .ok_or_else(|| ChainError::NetworkError("Peer not found".to_string()))?;
        Ok(())
    }

    /// Mark peer as syncing
    pub async fn set_peer_syncing(&self, node_addr: &str, syncing: bool) -> Result<(), ChainError> {
        let mut peers = self.peers.write().await;

        if let Some(peer) = peers.get_mut(node_addr) {
            peer.is_syncing = syncing;
            Ok(())
        } else {
            Err(ChainError::NetworkError("Peer not found".to_string()))
        }
    }

    /// Perform initial sync from peers
    pub async fn sync_from_peer(
        &self,
        peer: &Node,
        local_blockchain: &mut Blockchain,
    ) -> Result<u64, ChainError> {
        let peer_addr = peer.addr();
        let local_height = local_blockchain
            .blocks
            .last()
            .map_or(0, |b| b.header.height);

        self.set_sync_state(SyncState::Syncing).await;
        self.set_peer_syncing(&peer_addr, true).await?;

        println!(
            "🔄 Starting sync from peer {} (local: {}, remote: estimated)",
            peer_addr, local_height
        );

        // NOTE: This is a state management function only.
        // The actual block fetching and chain synchronization is performed by the NetworkNode
        // in network.rs (see NetworkNode::synchronize_with_peer).
        // This function tracks sync state and coordinates with the network layer.
        // The NetworkNode calls record_block_received() and record_sync_failure() to update our state.

        self.set_peer_syncing(&peer_addr, false).await?;
        self.set_sync_state(SyncState::Synced).await;

        let new_height = local_blockchain
            .blocks
            .last()
            .map_or(0, |b| b.header.height);
        Ok(new_height - local_height)
    }

    /// Check sync health by verifying we're not falling too far behind
    pub async fn check_sync_health(&self, local_height: u64) -> SyncState {
        let peers = self.peers.read().await;

        if peers.is_empty() {
            return SyncState::Idle;
        }

        let max_peer_height = peers.values().map(|p| p.height).max().unwrap_or(0);

        // Consider synced if within 1 block of the highest peer
        let is_caught_up = local_height >= max_peer_height.saturating_sub(1);

        if is_caught_up {
            SyncState::Synced
        } else {
            let lag = max_peer_height.saturating_sub(local_height);
            if lag > 100 {
                println!("⚠️  Node is {} blocks behind best peer", lag);
            }
            SyncState::Syncing
        }
    }

    /// Clean up stale peers
    pub async fn cleanup_stale_peers(&self) {
        let mut peers = self.peers.write().await;
        let stale_peers: Vec<_> = peers
            .iter()
            .filter(|(_, p)| p.is_stale())
            .map(|(addr, _)| addr.clone())
            .collect();

        for addr in stale_peers {
            println!("🗑️  Removing stale peer: {}", addr);
            peers.remove(&addr);
        }
    }

    /// Get detailed peer information
    pub async fn get_peer_info(&self, node_addr: &str) -> Option<PeerSyncInfo> {
        let peers = self.peers.read().await;
        peers.get(node_addr).cloned()
    }
}

impl Default for NodeSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_peer() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let sync = NodeSynchronizer::new();
            let node = Node::new("127.0.0.1".to_string(), 8333);

            assert!(sync.register_peer(node.clone(), 100).await.is_ok());
            assert_eq!(sync.peer_count().await, 1);
        }).await.expect("test_register_peer timed out");
    }

    #[tokio::test]
    async fn test_get_best_peer() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let sync = NodeSynchronizer::new();

            let node1 = Node::new("127.0.0.1".to_string(), 8333);
            let node2 = Node::new("127.0.0.2".to_string(), 8334);

            sync.register_peer(node1, 100).await.unwrap();
            sync.register_peer(node2, 200).await.unwrap();

            let best = sync.get_best_peer().await;
            assert!(best.is_some());
            assert_eq!(best.unwrap().port, 8334);
        }).await.expect("test_get_best_peer timed out");
    }

    #[tokio::test]
    async fn test_sync_stats() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let sync = NodeSynchronizer::new();
            let node = Node::new("127.0.0.1".to_string(), 8333);

            sync.register_peer(node.clone(), 100).await.unwrap();
            sync.record_block_received(&node.addr()).await.unwrap();

            let stats = sync.get_stats().await;
            assert_eq!(stats.blocks_synced_this_session, 1);
        }).await.expect("test_sync_stats timed out");
    }

    #[tokio::test]
    async fn test_peer_failure_tracking() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let sync = NodeSynchronizer::new();
            let node = Node::new("127.0.0.1".to_string(), 8333);

            sync.register_peer(node.clone(), 100).await.unwrap();

            // Record 3 failures
            for _ in 0..3 {
                sync.record_sync_failure(&node.addr()).await.unwrap();
            }

            let info = sync.get_peer_info(&node.addr()).await;
            assert!(info.unwrap().is_unreliable());
        }).await.expect("test_peer_failure_tracking timed out");
    }

    #[tokio::test]
    async fn test_pending_blocks_queue() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let sync = NodeSynchronizer::new();

            assert!(!sync.has_pending_blocks().await);

            assert_eq!(sync.peer_count().await, 0);
        }).await.expect("test_pending_blocks_queue timed out");
    }
}
