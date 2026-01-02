//! Production-grade REST API server for TrinityChain
//!
//! Provides secure, rate-limited HTTP endpoints for blockchain interaction,
//! mining control, network management, and wallet operations.

use axum::{
    extract::{Path, Query, Request, State},
    http::{self, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use hex::decode_to_slice;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

use crate::blockchain::{Block, Blockchain, Sha256Hash};
use crate::crypto::KeyPair;
use crate::error::ChainError;
use crate::geometry::Coord;
use crate::miner;
use crate::network::NetworkNode;
use crate::transaction::{CoinbaseTx, Transaction};

// API Configuration
const DEFAULT_API_PORT: u16 = 3000;

// Reserved for future use
#[allow(dead_code)]
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
#[allow(dead_code)]
const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB
#[allow(dead_code)]
const RATE_LIMIT_REQUESTS: u32 = 100;
#[allow(dead_code)]
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Node state with mining capabilities
#[derive(Clone)]
pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub network: Arc<NetworkNode>,
    // Optional shared orchestrator state (NodeState) for health checks and logging
    pub state: Option<Arc<RwLock<crate::node::NodeState>>>,
    is_mining: Arc<AtomicBool>,
    blocks_mined: Arc<AtomicU64>,
    mining_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    api_stats: Arc<RwLock<ApiStats>>,
}

/// API statistics and monitoring
#[derive(Debug, Default)]
struct ApiStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    mining_starts: u64,
    mining_stops: u64,
    transactions_submitted: u64,
    start_time: Option<Instant>,
}

impl ApiStats {
    fn new() -> Self {
        ApiStats {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    fn record_request(&mut self, success: bool) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
    }
}

/// Rate limiter for API endpoints (reserved for future implementation)
#[allow(dead_code)]
#[derive(Debug)]
struct RateLimiter {
    requests: HashMap<String, (u32, Instant)>,
}

#[allow(dead_code)]
impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            requests: HashMap::new(),
        }
    }

    fn check_rate_limit(&mut self, identifier: &str) -> Result<(), ApiError> {
        let now = Instant::now();

        // Clean up old entries
        self.requests
            .retain(|_, (_, timestamp)| now.duration_since(*timestamp) < RATE_LIMIT_WINDOW);

        let entry = self
            .requests
            .entry(identifier.to_string())
            .or_insert((0, now));

        if now.duration_since(entry.1) >= RATE_LIMIT_WINDOW {
            // Reset window
            entry.0 = 0;
            entry.1 = now;
        }

        if entry.0 >= RATE_LIMIT_REQUESTS {
            return Err(ApiError::RateLimitExceeded);
        }

        entry.0 += 1;
        Ok(())
    }
}

impl Node {
    /// Create a new node instance
    pub fn new(blockchain: Blockchain) -> Self {
        let blockchain_arc = Arc::new(RwLock::new(blockchain));
        let network_arc = Arc::new(NetworkNode::new(blockchain_arc.clone()));

        Self {
            blockchain: blockchain_arc,
            network: network_arc,
            state: None,
            is_mining: Arc::new(AtomicBool::new(false)),
            blocks_mined: Arc::new(AtomicU64::new(0)),
            mining_task: Arc::new(RwLock::new(None)),
            api_stats: Arc::new(RwLock::new(ApiStats::new())),
        }
    }

    /// Create a new API node that shares the provided blockchain and network
    /// instances. This is useful for integrating the API server with the
    /// authoritative `trinity-node` orchestrator so both services observe the
    /// same in-memory chain and peer list.
    pub fn new_shared(
        blockchain: Arc<RwLock<Blockchain>>,
        network: Arc<NetworkNode>,
        state: Option<Arc<RwLock<crate::node::NodeState>>>,
    ) -> Self {
        Self {
            blockchain: blockchain.clone(),
            network: network.clone(),
            state: state.clone(),
            is_mining: Arc::new(AtomicBool::new(false)),
            blocks_mined: Arc::new(AtomicU64::new(0)),
            mining_task: Arc::new(RwLock::new(None)),
            api_stats: Arc::new(RwLock::new(ApiStats::new())),
        }
    }

    /// Check if currently mining
    pub fn is_mining(&self) -> bool {
        self.is_mining.load(Ordering::Relaxed)
    }

    /// Get total blocks mined
    pub fn blocks_mined(&self) -> u64 {
        self.blocks_mined.load(Ordering::Relaxed)
    }

    /// Start mining with proper validation and error handling
    pub async fn start_mining(&self, miner_address: String) -> Result<(), ApiError> {
        // Validate address format
        if miner_address.is_empty() {
            return Err(ApiError::InvalidInput(
                "Miner address cannot be empty".to_string(),
            ));
        }

        let mut address = [0u8; 32];
        hex::decode_to_slice(&miner_address, &mut address)
            .map_err(|e| ApiError::InvalidInput(format!("Invalid miner address: {}", e)))?;

        // Check if already mining
        if self
            .is_mining
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(ApiError::MiningAlreadyRunning);
        }

        // Update stats
        {
            let mut stats = self.api_stats.write().await;
            stats.mining_starts += 1;
        }

        let node_clone = self.clone();
        let task = tokio::spawn(async move {
            println!("Mining started for address: {}", miner_address);

            loop {
                if !node_clone.is_mining.load(Ordering::Relaxed) {
                    break;
                }

                let new_block = {
                    let bc = node_clone.blockchain.read().await;

                    if bc.blocks.is_empty() {
                        eprintln!("Cannot mine without a genesis block.");
                        break;
                    }

                    let last_block = match bc.blocks.last() {
                        Some(b) => b.clone(),
                        None => {
                            eprintln!("Cannot determine last block: chain appears empty");
                            break;
                        }
                    };
                    let transactions = bc.mempool.get_all_transactions();
                    let height = bc.blocks.len() as u64;
                    let reward = Blockchain::calculate_block_reward(height);

                    let mut address = [0u8; 32];
                    if let Err(e) = hex::decode_to_slice(&miner_address, &mut address) {
                        eprintln!("Invalid miner address while mining: {}", e);
                        break;
                    }

                    let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
                        reward_area: Coord::from_num(reward),
                        beneficiary_address: address,
                        nonce: 0,
                    });

                    let mut all_txs = vec![coinbase_tx];
                    all_txs.extend(transactions);

                    Some(Block::new(
                        height,
                        last_block.hash(),
                        bc.difficulty,
                        all_txs,
                    ))
                };

                if let Some(block) = new_block {
                    match miner::mine_block(block) {
                        Ok(mined_block) => {
                            let mut bc = node_clone.blockchain.write().await;
                            match bc.apply_block(mined_block.clone()) {
                                Ok(_) => {
                                    node_clone.blocks_mined.fetch_add(1, Ordering::SeqCst);
                                    node_clone.network.broadcast_block(&mined_block).await;
                                    println!(
                                        "✅ Successfully mined block at height {}",
                                        mined_block.header.height
                                    );
                                }
                                Err(e) => {
                                    eprintln!("❌ Mined block was invalid: {}", e);
                                    // Continue mining despite this error
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Mining error: {}", e);
                            // Small delay before retrying
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                } else {
                    break;
                }

                // Small delay between mining attempts
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            node_clone.is_mining.store(false, Ordering::SeqCst);
            println!("Mining has stopped.");
        });

        *self.mining_task.write().await = Some(task);
        Ok(())
    }

    /// Stop mining gracefully
    pub async fn stop_mining(&self) -> Result<(), ApiError> {
        if self
            .is_mining
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(ApiError::MiningNotRunning);
        }

        // Update stats
        {
            let mut stats = self.api_stats.write().await;
            stats.mining_stops += 1;
        }

        println!("Stopping mining...");

        if let Some(task) = self.mining_task.write().await.take() {
            // Give the task a moment to stop gracefully
            tokio::time::sleep(Duration::from_millis(100)).await;

            if !task.is_finished() {
                task.abort();
                println!("Mining task aborted.");
            }
        }

        Ok(())
    }

    /// Get API statistics
    pub async fn get_stats(&self) -> ApiStatsResponse {
        let stats = self.api_stats.read().await;
        let uptime = stats.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);

        ApiStatsResponse {
            total_requests: stats.total_requests,
            successful_requests: stats.successful_requests,
            failed_requests: stats.failed_requests,
            mining_starts: stats.mining_starts,
            mining_stops: stats.mining_stops,
            transactions_submitted: stats.transactions_submitted,
            uptime_seconds: uptime,
            blocks_mined: self.blocks_mined(),
            is_mining: self.is_mining(),
        }
    }
}

// ============================================================================
// API Error Handling
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BlockchainError(ChainError),
    InvalidInput(String),
    NotFound(String),
    MiningAlreadyRunning,
    MiningNotRunning,
    RateLimitExceeded,
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BlockchainError(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            ApiError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::MiningAlreadyRunning => (
                StatusCode::CONFLICT,
                "Mining is already running".to_string(),
            ),
            ApiError::MiningNotRunning => {
                (StatusCode::CONFLICT, "Mining is not running".to_string())
            }
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

impl From<ChainError> for ApiError {
    fn from(err: ChainError) -> Self {
        ApiError::BlockchainError(err)
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: String, // Changed to String to preserve floating-point precision of Coord
    pub address: String,
}

// Struct to hold a transaction and its containing block height
#[derive(Serialize)]
pub struct TransactionHistoryEntry {
    pub transaction: Transaction,
    pub block_height: u64,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub height: u64,
    pub difficulty: u32,
    pub mempool_size: usize,
    pub total_blocks: u64,
}

#[derive(Serialize)]
pub struct ApiStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub mining_starts: u64,
    pub mining_stops: u64,
    pub transactions_submitted: u64,
    pub uptime_seconds: u64,
    pub blocks_mined: u64,
    pub is_mining: bool,
}

#[derive(Deserialize)]
pub struct StartMiningRequest {
    pub miner_address: String,
}

#[derive(Serialize)]
struct WalletResponse {
    address: String,
    public_key: String,
    private_key: String,
}

#[derive(Serialize)]
struct SuccessResponse {
    message: String,
}

#[derive(Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_page")]
    page: u64,
    #[serde(default = "default_limit")]
    limit: u64,
}

fn default_page() -> u64 {
    0
}
fn default_limit() -> u64 {
    10
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Parses a 64-character hex string into a Sha256Hash ([u8; 32]).
fn parse_hash(hash_str: &str) -> Result<Sha256Hash, ApiError> {
    if hash_str.len() != 64 {
        return Err(ApiError::InvalidInput(
            "Hash must be a 64-character hex string".to_string(),
        ));
    }
    let mut hash_bytes = [0u8; 32];
    decode_to_slice(hash_str, &mut hash_bytes)
        .map_err(|e| ApiError::InvalidInput(format!("Invalid hex hash: {}", e)))?;
    Ok(hash_bytes)
}

// ============================================================================
// Middleware
// ============================================================================

/// Request logging and statistics middleware
async fn stats_middleware(State(node): State<Arc<Node>>, req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    let success = response.status().is_success();
    let mut stats = node.api_stats.write().await;
    stats.record_request(success);

    response
}

/// Detailed request logging middleware. Logs method, path, status, duration
/// and current `NodeState` (when available).
async fn logging_middleware(
    State(node): State<Arc<Node>>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    let node_state = if let Some(s) = &node.state {
        format!("{:?}", s.read().await.clone())
    } else {
        "unknown".to_string()
    };

    tracing::info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        node_state = %node_state,
        "api.request"
    );

    response
}

// ============================================================================
// API Server
// ============================================================================

/// Build the API router with all endpoints (for testing)
pub fn build_api_router(node: Arc<Node>) -> Router {
    // CORS configuration - allow all origins with credentials
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request()) // Reflect the request's origin
        .allow_methods(vec![
            http::Method::GET,
            http::Method::POST,
            http::Method::OPTIONS,
        ]) // Explicitly allow methods
        .allow_headers(vec![http::header::CONTENT_TYPE]) // Explicitly allow headers
        .allow_credentials(true);

    // API routes
    let api_routes = Router::new()
        // Blockchain endpoints
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/blocks", get(get_blocks))
        .route("/blockchain/block/:height", get(get_block_by_height))
        .route("/blockchain/stats", get(get_blockchain_stats))
        // Transaction endpoints
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:hash", get(get_transaction))
        .route("/mempool", get(get_mempool))
        // Mining endpoints
        .route("/mining/start", post(start_mining))
        .route("/mining/stop", post(stop_mining))
        .route("/mining/status", get(get_mining_status))
        // Network endpoints
        .route("/network/peers", get(get_peers))
        .route("/network/info", get(get_network_info))
        // Address endpoints
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/address/:addr/transactions", get(get_address_transactions))
        // Wallet endpoints
        .route("/wallet/create", post(create_wallet))
        // System endpoints
        .route("/health", get(health_check))
        .route("/stats", get(get_api_stats))
        // logging before stats so we always record timing and node-state
        .layer(middleware::from_fn_with_state(node.clone(), logging_middleware))
        .layer(middleware::from_fn_with_state(
            node.clone(),
            stats_middleware,
        ))
        .with_state(node)
        .layer(cors.clone());

    // Serve static dashboard files
    let serve_dir = ServeDir::new("dashboard/dist");
    Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(cors)
}

/// Run the API server with production-grade configuration
pub async fn run_api_server(node: Arc<Node>) -> Result<(), Box<dyn std::error::Error>> {
    // CORS configuration - allow all origins with credentials
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request()) // Reflect the request's origin
        .allow_methods(vec![
            http::Method::GET,
            http::Method::POST,
            http::Method::OPTIONS,
        ]) // Explicitly allow methods
        .allow_headers(vec![http::header::CONTENT_TYPE]) // Explicitly allow headers
        .allow_credentials(true);

    // API routes
    let api_routes = Router::new()
        // Blockchain endpoints
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/blocks", get(get_blocks))
        .route("/blockchain/block/:height", get(get_block_by_height))
        .route("/blockchain/stats", get(get_blockchain_stats))
        // Transaction endpoints
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:hash", get(get_transaction))
        .route("/mempool", get(get_mempool))
        // Mining endpoints
        .route("/mining/start", post(start_mining))
        .route("/mining/stop", post(stop_mining))
        .route("/mining/status", get(get_mining_status))
        // Network endpoints
        .route("/network/peers", get(get_peers))
        .route("/network/info", get(get_network_info))
        // Address endpoints
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/address/:addr/transactions", get(get_address_transactions))
        // Wallet endpoints
        .route("/wallet/create", post(create_wallet))
        // System endpoints
        .route("/health", get(health_check))
        .route("/stats", get(get_api_stats))
        // logging before stats so we always record timing and node-state
        .layer(middleware::from_fn_with_state(node.clone(), logging_middleware))
        .layer(middleware::from_fn_with_state(
            node.clone(),
            stats_middleware,
        ))
        .with_state(node)
        .layer(cors.clone());

    // Serve static dashboard files
    let serve_dir = ServeDir::new("dashboard/dist");
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(cors);

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(DEFAULT_API_PORT);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Binds the TCP listener to the address, defining the 'listener' variable
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("🚀 API server listening on http://{}", addr);
    println!("📊 Dashboard available at http://{}", addr);
    println!("🔗 API documentation at http://{}/api", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// Route Handlers
// ============================================================================

async fn health_check(State(node): State<Arc<Node>>) -> impl IntoResponse {
    // If the orchestrator provided a `NodeState`, use it to determine health.
    if let Some(s) = &node.state {
        let state = s.read().await.clone();
        match state {
            crate::node::NodeState::Ready => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "healthy",
                    "node_state": format!("{:?}", state),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            )
                .into_response(),
            _ => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "unhealthy",
                    "node_state": format!("{:?}", state),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            )
                .into_response(),
        }
    } else {
        // No orchestrator state available — assume healthy
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
            .into_response()
    }
}

async fn get_blockchain_height(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    Json(blockchain.blocks.len() as u64)
}

fn hash_to_hex(hash: &Sha256Hash) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn get_blocks(
    State(node): State<Arc<Node>>,
    Query(params): Query<PaginationQuery>,
) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    let total = blockchain.blocks.len();

    let limit = params.limit.min(100); // Max 100 blocks per request
    let offset = params.page * limit;

    if offset >= total as u64 {
        return Json(serde_json::json!({
            "blocks": [],
            "total": total,
            "page": params.page,
            "limit": limit
        }));
    }

    let blocks_json: Vec<_> = blockchain
        .blocks
        .iter()
        .rev()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|b| {
            let reward = b
                .transactions
                .iter()
                .find_map(|tx| {
                    if let crate::transaction::Transaction::Coinbase(cb) = tx {
                        Some(cb.reward_area.to_num::<f64>())
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);
            serde_json::json!({
                "index": b.header.height,
                "timestamp": b.header.timestamp,
                "hash": hash_to_hex(&b.hash()),
                "previous_hash": hash_to_hex(&b.header.previous_hash),
                "difficulty": b.header.difficulty,
                "nonce": b.header.nonce,
                "transactions": b.transactions,
                "reward": reward
            })
        })
        .collect();

    Json(serde_json::json!({
        "blocks": blocks_json,
        "total": total,
        "page": params.page,
        "limit": limit
    }))
}

async fn get_block_by_height(
    State(node): State<Arc<Node>>,
    Path(height): Path<u64>,
) -> Result<Json<Block>, ApiError> {
    let blockchain = node.blockchain.read().await;

    blockchain
        .blocks
        .get(height as usize)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("Block at height {} not found", height)))
        .map(Json)
}

async fn get_blockchain_stats(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    let stats = StatsResponse {
        height: blockchain.blocks.len() as u64,
        difficulty: blockchain.difficulty,
        mempool_size: blockchain.mempool.len(),
        total_blocks: blockchain.blocks.len() as u64,
    };
    Json(stats)
}

async fn get_mempool(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    let transactions = blockchain.mempool.get_all_transactions();
    Json(serde_json::json!({
        "count": transactions.len(),
        "transactions": transactions
    }))
}

async fn submit_transaction(
    State(node): State<Arc<Node>>,
    Json(tx): Json<Transaction>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let mut blockchain = node.blockchain.write().await;

    blockchain.mempool.add_transaction(tx.clone())?;

    // Update stats
    {
        let mut stats = node.api_stats.write().await;
        stats.transactions_submitted += 1;
    }

    // Broadcast to network
    node.network.broadcast_transaction(&tx).await;

    Ok(Json(SuccessResponse {
        message: "Transaction submitted successfully".to_string(),
    }))
}

async fn get_transaction(
    State(node): State<Arc<Node>>,
    Path(hash_str): Path<String>,
) -> Result<Json<Transaction>, ApiError> {
    let target_hash = parse_hash(&hash_str)?;
    let blockchain = node.blockchain.read().await;

    // 1. Search in blocks (on-chain)
    for block in &blockchain.blocks {
        for tx in &block.transactions {
            if tx.hash() == target_hash {
                return Ok(Json(tx.clone()));
            }
        }
    }

    // 2. Search in mempool (unconfirmed)
    if let Some(tx) = blockchain.mempool.get_transaction(&target_hash) {
        return Ok(Json(tx.clone()));
    }

    Err(ApiError::NotFound(format!(
        "Transaction {} not found",
        hash_str
    )))
}

async fn start_mining(
    State(node): State<Arc<Node>>,
    Json(req): Json<StartMiningRequest>,
) -> Result<Json<SuccessResponse>, ApiError> {
    node.start_mining(req.miner_address).await?;

    Ok(Json(SuccessResponse {
        message: "Mining started successfully".to_string(),
    }))
}

async fn stop_mining(State(node): State<Arc<Node>>) -> Result<Json<SuccessResponse>, ApiError> {
    node.stop_mining().await?;

    Ok(Json(SuccessResponse {
        message: "Mining stopped successfully".to_string(),
    }))
}

async fn get_mining_status(State(node): State<Arc<Node>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "is_mining": node.is_mining(),
        "blocks_mined": node.blocks_mined()
    }))
}

async fn get_peers(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let peers = node.network.list_peers().await;
    Json(serde_json::json!({
        "count": peers.len(),
        "peers": peers
    }))
}

async fn get_network_info(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let peers = node.network.list_peers().await;
    Json(serde_json::json!({
        "peer_count": peers.len(),
        "peers": peers,
        "protocol_version": "1.0"
    }))
}

async fn get_address_balance(
    State(node): State<Arc<Node>>,
    Path(addr_str): Path<String>,
) -> impl IntoResponse {
    let mut addr = [0u8; 32];
    if hex::decode_to_slice(&addr_str, &mut addr).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid address format".to_string(),
            }),
        )
            .into_response();
    }

    let blockchain = node.blockchain.read().await;
    // Format the balance (Coord) as a String to preserve floating-point precision
    let balance = format!("{}", blockchain.state.get_balance(&addr));

    Json(BalanceResponse {
        balance, // Now a String
        address: addr_str,
    })
    .into_response()
}

async fn get_address_transactions(
    State(node): State<Arc<Node>>,
    Path(addr_str): Path<String>,
) -> impl IntoResponse {
    let mut target_addr = [0u8; 32];
    if hex::decode_to_slice(&addr_str, &mut target_addr).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid address format".to_string(),
            }),
        )
            .into_response();
    }

    let blockchain = node.blockchain.read().await;

    // We will collect transactions found on the blockchain and transactions found in the mempool.
    let mut transactions: Vec<TransactionHistoryEntry> = Vec::new();

    // 1. Search confirmed transactions in the blockchain
    // We iterate backwards from the latest block for common chronological display in wallets.
    for block in blockchain.blocks.iter().rev() {
        let block_height = block.header.height;
        for tx in &block.transactions {
            let matches = match tx {
                // Transfer transaction: involves sender (input) or new_owner (output)
                Transaction::Transfer(transfer_tx) => {
                    transfer_tx.sender == target_addr || transfer_tx.new_owner == target_addr
                }
                // Subdivision transaction: check if the target address is the owner performing the split
                Transaction::Subdivision(subdivision_tx) => {
                    subdivision_tx.owner_address == target_addr
                }
                // Coinbase transactions only have a beneficiary (miner)
                Transaction::Coinbase(coinbase_tx) => {
                    coinbase_tx.beneficiary_address == target_addr
                }
            };

            if matches {
                transactions.push(TransactionHistoryEntry {
                    transaction: tx.clone(),
                    block_height,
                });
            }
        }
    }

    // 2. Search unconfirmed transactions in the mempool
    // These entries will have a block_height of 0 (unconfirmed)
    for tx in blockchain.mempool.get_all_transactions() {
        let matches = match &tx {
            // Transfer transaction: involves sender (input) or new_owner (output)
            Transaction::Transfer(transfer_tx) => {
                transfer_tx.sender == target_addr || transfer_tx.new_owner == target_addr
            }
            // Subdivision transaction: check if the target address is the owner performing the split
            Transaction::Subdivision(subdivision_tx) => subdivision_tx.owner_address == target_addr,
            // Coinbase transactions are never in the mempool
            Transaction::Coinbase(_) => false,
        };

        if matches {
            // Unconfirmed transactions are assigned height 0
            transactions.push(TransactionHistoryEntry {
                transaction: tx.clone(),
                block_height: 0,
            });
        }
    }

    Json(serde_json::json!({
        "address": addr_str,
        "count": transactions.len(),
        "transactions": transactions,
    }))
    .into_response()
}

async fn create_wallet() -> Result<Json<WalletResponse>, ApiError> {
    let keypair = KeyPair::generate()
        .map_err(|e| ApiError::InternalError(format!("Failed to generate keypair: {}", e)))?;

    Ok(Json(WalletResponse {
        address: hex::encode(keypair.address()),
        public_key: hex::encode(keypair.public_key.serialize()),
        private_key: hex::encode(keypair.secret_key.as_ref()),
    }))
}

async fn get_api_stats(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let stats = node.get_stats().await;
    Json(stats)
}
