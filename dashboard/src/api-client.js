/**
 * TrinityChain API Client
 * Handles all REST API interactions for the dashboard
 */

class TrinityChainAPI {
  constructor(nodeUrl = 'http://localhost:3000') {
    this.nodeUrl = nodeUrl.replace(/\/$/, ''); // Remove trailing slash
  }

  setNodeUrl(url) {
    this.nodeUrl = url.replace(/\/$/, '');
  }

  async request(endpoint, options = {}) {
    const url = `${this.nodeUrl}${endpoint}`;
    const defaultOptions = {
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include', // Include cookies/auth for Codespaces forwarded ports
      ...options,
    };

    try {
      const response = await fetch(url, defaultOptions);
      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(error.error || `HTTP ${response.status}: ${response.statusText}`);
      }
      return await response.json();
    } catch (error) {
      throw new Error(`API Error: ${error.message}`);
    }
  }

  // ============================================================================
  // BLOCKCHAIN ENDPOINTS
  // ============================================================================

  async getBlockchainHeight() {
    return this.request('/api/blockchain/height');
  }

  async getBlocks(page = 0, limit = 10) {
    return this.request(`/api/blockchain/blocks?page=${page}&limit=${limit}`);
  }

  async getBlock(height) {
    return this.request(`/api/blockchain/block/${height}`);
  }

  async getBlockchainStats() {
    return this.request('/api/blockchain/stats');
  }

  // ============================================================================
  // TRANSACTION ENDPOINTS
  // ============================================================================

  async submitTransaction(transaction) {
    return this.request('/api/transaction', {
      method: 'POST',
      body: JSON.stringify(transaction),
    });
  }

  async getTransaction(hash) {
    return this.request(`/api/transaction/${hash}`);
  }

  async getMempool() {
    return this.request('/api/mempool');
  }

  // ============================================================================
  // NETWORK ENDPOINTS
  // ============================================================================

  async getNetworkPeers() {
    return this.request('/api/network/peers');
  }

  async getNetworkInfo() {
    return this.request('/api/network/info');
  }

  // ============================================================================
  // ADDRESS & BALANCE ENDPOINTS
  // ============================================================================

  async getBalance(address) {
    return this.request(`/api/address/${address}/balance`);
  }

  async getAddressTransactions(address) {
    return this.request(`/api/address/${address}/transactions`);
  }

  // ============================================================================
  // WALLET ENDPOINTS
  // ============================================================================

  async createWallet() {
    return this.request('/api/wallet/create', {
      method: 'POST',
    });
  }

  // ============================================================================
  // SYSTEM ENDPOINTS
  // ============================================================================

  async healthCheck() {
    return this.request('/health');
  }

  async getStats() {
    return this.request('/stats');
  }

  // ============================================================================
  // UTILITY METHODS (localStorage based - for client-side wallet management)
  // ============================================================================

  // Save wallet to localStorage
  saveWallet(name, wallet) {
    const wallets = this.getAllWallets();
    wallets[name] = wallet;
    localStorage.setItem('trinity_wallets', JSON.stringify(wallets));
  }

  // Get wallet from localStorage
  getWallet(name) {
    const wallets = this.getAllWallets();
    return wallets[name] || null;
  }

  // Get all wallets from localStorage
  getAllWallets() {
    const stored = localStorage.getItem('trinity_wallets');
    return stored ? JSON.parse(stored) : {};
  }

  // List all wallet names
  listWalletNames() {
    return Object.keys(this.getAllWallets());
  }

  // Delete wallet from localStorage
  deleteWallet(name) {
    const wallets = this.getAllWallets();
    delete wallets[name];
    localStorage.setItem('trinity_wallets', JSON.stringify(wallets));
  }

  // Export wallet as JSON
  exportWallet(name) {
    const wallet = this.getWallet(name);
    if (!wallet) throw new Error(`Wallet ${name} not found`);
    return JSON.stringify({ [name]: wallet }, null, 2);
  }

  // Import wallet from JSON
  importWallet(name, walletJson) {
    try {
      const parsed = JSON.parse(walletJson);
      const wallet = parsed[name] || Object.values(parsed)[0];
      if (!wallet) throw new Error('No wallet data found in import');
      this.saveWallet(name, wallet);
      return wallet;
    } catch (error) {
      throw new Error(`Failed to import wallet: ${error.message}`);
    }
  }

  // Helper to format address display
  formatAddress(address, length = 42) {
    if (!address) return 'N/A';
    if (address.length <= length) return address;
    const start = Math.floor((length - 3) / 2);
    const end = length - 3 - start;
    return `${address.slice(0, start)}...${address.slice(-end)}`;
  }

  // Helper to format numbers
  formatNumber(num) {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return num.toFixed(2);
  }

  // Helper to format hash
  formatHash(hash, length = 20) {
    if (!hash || hash.length <= length) return hash;
    const start = Math.floor((length - 3) / 2);
    const end = length - 3 - start;
    return `${hash.slice(0, start)}...${hash.slice(-end)}`;
  }
}

export default TrinityChainAPI;
