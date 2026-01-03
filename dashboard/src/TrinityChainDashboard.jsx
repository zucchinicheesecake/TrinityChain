import React, { useState, useEffect } from 'react';
import { Activity, Boxes, Clock, Award, Database, Network as NetworkIcon, BarChart3, Terminal } from 'lucide-react';
import Header from './components/Header';
import { HeroStatCard } from './components/StatCard';
import Analytics from './components/Analytics';
import Network from './components/Network';
import BlockExplorer from './components/BlockExplorer';
import DiagnosticTerminal from './DiagnosticTerminal';
import NetworkManager from './NetworkManager';

const TrinityChainDashboard = () => {
  const [stats, setStats] = useState(null);
  const [blocks, setBlocks] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const getNodeUrl = () => {
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
      return 'http://localhost:3000';
    }
    if (window.location.hostname.match(/^\d+\.\d+\.\d+\.\d+$/)) {
      return 'http://localhost:3000';
    }
    if (window.location.hostname.includes('.github.dev')) {
      return window.location.origin.replace('-5173.', '-3000.');
    }
    if (window.location.hostname.includes('render.com') || window.location.hostname.includes('vercel.app')) {
      return window.location.origin;
    }
    return `${window.location.protocol}//${window.location.hostname}:3000`;
  };
  const [nodeUrl, setNodeUrl] = useState(getNodeUrl());
  const [activeTab, setActiveTab] = useState('dashboard');
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(3000);
  const [chartData, setChartData] = useState([]);
  const [networkData, setNetworkData] = useState([]);
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
    fetchData();
    if (autoRefresh) {
      const interval = setInterval(fetchData, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [nodeUrl, autoRefresh, refreshInterval]);

  const fetchData = async () => {
    try {
      const [statsRes, blocksRes] = await Promise.all([
        fetch(`${nodeUrl}/api/blockchain/stats`, { credentials: 'include' }),
        fetch(`${nodeUrl}/api/blockchain/blocks?limit=50`, { credentials: 'include' }),
      ]);

      if (!statsRes.ok || !blocksRes.ok) {
        throw new Error(`HTTP ${statsRes.status}/${blocksRes.status}`);
      }

      const statsData = await statsRes.json();
      const blocksData = await blocksRes.json();

      const mappedStats = {
        chainHeight: statsData.height || 0,
        difficulty: statsData.difficulty || 0,
        mempoolSize: statsData.mempool_size || 0,
        totalBlocks: statsData.total_blocks || 0,
        currentReward: 0,
        avgBlockTime: 10,
        uptime: 0,
        totalSupply: 0,
        maxSupply: 420000000,
      };

      setStats(mappedStats);
      setBlocks(blocksData.blocks || []);

      const recentBlocks = (blocksData.blocks || []).slice(0, 20).reverse();
      const newChartData = recentBlocks.map((block, idx) => ({
        block: block.index,
        difficulty: block.difficulty,
        transactions: block.transactions?.length || 0,
        reward: block.reward || 0,
        time: idx,
      }));
      setChartData(newChartData);

      if (recentBlocks.length > 1) {
        const networkPerf = recentBlocks
          .slice(0, 10)
          .map((block, idx) => {
            const prevBlock = recentBlocks[idx + 1];
            const blockTime = prevBlock ? (new Date(block.timestamp) - new Date(prevBlock.timestamp)) / 1000 : 0;
            return {
              block: block.index,
              blockTime: Math.max(0, blockTime),
              hashrate: (block.difficulty * 1000) / Math.max(blockTime, 0.1),
            };
          })
          .reverse();
        setNetworkData(networkPerf);
      }

      setError(null);
      setLoading(false);
    } catch (err) {
      setError(err.message);
      setLoading(false);
    }
  };

  const formatNumber = (num) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return new Intl.NumberFormat('en-US').format(num);
  };

  const formatFullNumber = (num) => {
    return new Intl.NumberFormat('en-US').format(num);
  };

  const formatTime = (seconds) => {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (mins < 60) return `${mins}m ${secs}s`;
    const hours = Math.floor(mins / 60);
    const remainMins = mins % 60;
    return `${hours}h ${remainMins}m`;
  };

  const formatHash = (hash) => {
    if (!hash) return 'N/A';
    return `${hash.slice(0, 12)}...${hash.slice(-10)}`;
  };

  const calculatePercentage = (current, total) => {
    return ((current / total) * 100).toFixed(3);
  };

  const calculateHashrate = () => {
    if (!stats || !stats.difficulty || !stats.avgBlockTime) return 0;
    return (stats.difficulty * 1000 / Math.max(stats.avgBlockTime, 0.1)).toFixed(2);
  };

  if (loading && !stats && !error) {
    return (
      <div className="min-h-screen bg-dark-purple flex items-center justify-center">
        <div className="text-center">
          <div className="relative w-24 h-24 mx-auto mb-6">
            <div className="absolute inset-0 border-4 border-neon-pink/30 rounded-full"></div>
            <div className="absolute inset-0 border-4 border-transparent border-t-neon-pink rounded-full animate-spin"></div>
            <img src="/logo.png" alt="TrinityChain" className="absolute inset-0 m-auto w-12 h-12 object-contain" />
          </div>
          <p className="text-neon-cyan text-xl font-semibold soft-glow">Connecting to TrinityChain Node...</p>
          <p className="text-neon-pink text-sm mt-2">{nodeUrl}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-dark-purple text-white">
      <Header
        autoRefresh={autoRefresh}
        setAutoRefresh={setAutoRefresh}
        fetchData={fetchData}
        loading={loading}
        showSettings={showSettings}
        setShowSettings={setShowSettings}
        error={error}
        nodeUrl={nodeUrl}
        setNodeUrl={setNodeUrl}
        refreshInterval={refreshInterval}
        setRefreshInterval={setRefreshInterval}
      />
      <div className="max-w-7xl mx-auto p-6">
        {error && (
          <div className="glass-card neon-border border-red-500 rounded-lg p-4 mb-6">
            <div className="flex items-center gap-3">
              <div className="bg-red-500/20 rounded-full p-2">
                <Activity className="text-red-400" size={20} />
              </div>
              <div>
                <p className="text-red-200 font-semibold">Connection Error</p>
                <p className="text-red-300 text-sm">
                  {error} - Ensure TrinityChain node is running on {nodeUrl}
                </p>
              </div>
            </div>
          </div>
        )}
        <div className="flex gap-2 mb-6 overflow-x-auto pb-2">
          {[
            { id: 'dashboard', label: 'Dashboard', icon: BarChart3 },
            { id: 'network', label: 'Network', icon: NetworkIcon },
            { id: 'analytics', label: 'Analytics', icon: Activity },
            { id: 'explorer', label: 'Block Explorer', icon: Boxes },
            { id: 'terminal', label: 'Terminal', icon: Terminal },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-6 py-3 rounded-lg font-semibold transition-all whitespace-nowrap glass-card ${
                activeTab === tab.id
                  ? 'bg-gradient-to-r from-neon-pink to-neon-cyan text-white shadow-neon-glow'
                  : 'text-neon-cyan hover:bg-dark-purple border border-neon-cyan/20'
              }`}
            >
              <tab.icon size={18} />
              {tab.label}
            </button>
          ))}
        </div>
        {activeTab === 'network' && (
          <div className="space-y-6">
            <NetworkManager nodeUrl={nodeUrl} />
            <Network
              stats={stats}
              networkData={networkData}
              calculateHashrate={calculateHashrate}
              formatNumber={formatNumber}
            />
          </div>
        )}
        {activeTab === 'dashboard' && (
          <>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
              <HeroStatCard
                icon={<Boxes className="text-neon-cyan" size={28} />}
                label="Chain Height"
                value={stats ? formatFullNumber(stats.chainHeight || 0) : 'Loading...'}
                subtext={stats ? `Total Blocks: ${formatNumber(stats.totalBlocks || 0)}` : ''}
              />
              <HeroStatCard
                icon={<Clock className="text-neon-pink" size={28} />}
                label="Block Time"
                value={stats ? `${(stats.avgBlockTime || 0).toFixed(2)}s` : 'Loading...'}
                subtext={stats ? `Uptime: ${formatTime(stats.uptime || 0)}` : ''}
              />
            </div>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
              <div className="glass-card neon-border">
                <div className="flex items-center justify-between mb-4 p-6">
                  <div className="flex items-center gap-3">
                    <div className="bg-neon-pink/20 rounded-lg p-3">
                      <Database className="text-neon-pink" size={24} />
                    </div>
                    <div>
                      <h3 className="text-xl font-bold soft-glow">Token Supply</h3>
                      <p className="text-neon-cyan text-sm">Distribution Progress</p>
                    </div>
                  </div>
                </div>
                <div className="space-y-4 p-6">
                  <div className="flex justify-between items-end">
                    <div>
                      <p className="text-neon-cyan text-sm">Current Supply</p>
                      <p className="text-3xl font-bold soft-glow">{formatNumber(stats?.totalSupply || 0)}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-neon-cyan text-sm">Max Supply</p>
                      <p className="text-2xl font-bold soft-glow">
                        {formatNumber(stats?.maxSupply || 420000000)}
                      </p>
                    </div>
                  </div>
                  <div className="relative">
                    <div className="w-full bg-dark-purple rounded-full h-6 overflow-hidden">
                      <div
                        className="h-6 rounded-full bg-gradient-to-r from-neon-pink to-neon-cyan transition-all duration-1000 flex items-center justify-end pr-3"
                        style={{
                          width: `${Math.min(
                            100,
                            calculatePercentage(stats?.totalSupply || 0, stats?.maxSupply || 420000000)
                          )}%`,
                        }}
                      >
                        <span className="text-xs font-bold text-dark-purple drop-shadow-lg">
                          {calculatePercentage(stats?.totalSupply || 0, stats?.maxSupply || 420000000)}%
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="grid grid-cols-3 gap-3 pt-2">
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-cyan text-xs mb-1">Remaining</p>
                      <p className="font-bold text-sm soft-glow">
                        {formatNumber((stats?.maxSupply || 420000000) - (stats?.totalSupply || 0))}
                      </p>
                    </div>
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-cyan text-xs mb-1">Circulating</p>
                      <p className="font-bold text-sm soft-glow">{formatNumber(stats?.totalSupply || 0)}</p>
                    </div>
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-cyan text-xs mb-1">Burned</p>
                      <p className="font-bold text-sm soft-glow">0</p>
                    </div>
                  </div>
                </div>
              </div>
              <div className="glass-card neon-border">
                <div className="flex items-center justify-between mb-4 p-6">
                  <div className="flex items-center gap-3">
                    <div className="bg-neon-cyan/20 rounded-lg p-3">
                      <Award className="text-neon-cyan" size={24} />
                    </div>
                    <div>
                      <h3 className="text-xl font-bold soft-glow">Halving Schedule</h3>
                      <p className="text-neon-pink text-sm">Reward Reduction</p>
                    </div>
                  </div>
                </div>
                <div className="space-y-4 p-6">
                  <div className="flex justify-between items-end">
                    <div>
                      <p className="text-neon-pink text-sm">Current Era</p>
                      <p className="text-5xl font-bold bg-gradient-to-r from-neon-pink to-neon-cyan bg-clip-text text-transparent">
                        {stats?.halvingEra || 0}
                      </p>
                    </div>
                    <div className="text-right">
                      <p className="text-neon-pink text-sm">Current Reward</p>
                      <p className="text-3xl font-bold text-green-400">
                        {formatNumber(stats?.currentReward || 0)}
                      </p>
                    </div>
                  </div>
                  <div className="relative">
                    <div className="w-full bg-dark-purple rounded-full h-6 overflow-hidden">
                      <div
                        className="h-6 rounded-full bg-gradient-to-r from-neon-cyan to-neon-pink transition-all duration-1000 flex items-center justify-end pr-3"
                        style={{
                          width: `${Math.min(
                            100,
                            Math.max(0, 100 - ((stats?.blocksToHalving || 0) / 210000) * 100)
                          )}%`,
                        }}
                      >
                        <span className="text-xs font-bold text-dark-purple drop-shadow-lg">
                          {Math.max(
                            0,
                            100 - ((stats?.blocksToHalving || 0) / 210000) * 100
                          ).toFixed(1)}
                          %
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="grid grid-cols-3 gap-3 pt-2">
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-pink text-xs mb-1">Blocks Left</p>
                      <p className="font-bold text-sm soft-glow">
                        {formatNumber(stats?.blocksToHalving || 0)}
                      </p>
                    </div>
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-pink text-xs mb-1">Next Block</p>
                      <p className="font-bold text-sm soft-glow">
                        {formatNumber((stats?.chainHeight || 0) + (stats?.blocksToHalving || 0))}
                      </p>
                    </div>
                    <div className="glass-card p-3 text-center">
                      <p className="text-neon-pink text-xs mb-1">Next Reward</p>
                      <p className="font-bold text-sm soft-glow">
                        {formatNumber((stats?.currentReward || 1000) / 2)}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div className="glass-card neon-border p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="bg-neon-cyan/20 rounded-lg p-3">
                    <Boxes className="text-neon-cyan" size={24} />
                  </div>
                  <div>
                    <h3 className="text-xl font-bold soft-glow">Latest Blocks</h3>
                    <p className="text-neon-pink text-sm">Most recent blocks</p>
                  </div>
                </div>
              </div>
              <div className="space-y-2">
                {blocks.slice(0, 10).map((block, idx) => (
                  <div
                    key={idx}
                    className="glass-card p-4 hover:bg-dark-purple/70 transition-all border border-neon-cyan/10 hover:border-neon-cyan/30"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div className="bg-neon-cyan/20 rounded-lg px-3 py-2">
                          <span className="text-neon-cyan font-mono font-bold text-lg">
                            #{block.index}
                          </span>
                        </div>
                        <div>
                          <p className="text-sm text-white font-mono">
                            {formatHash(block.hash)}
                          </p>
                          <p className="text-xs text-neon-pink mt-1">
                            {new Date(block.timestamp).toLocaleString()}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-6">
                        <div className="text-right">
                          <p className="text-xs text-neon-cyan">Transactions</p>
                          <p className="font-bold soft-glow">{block.transactions?.length || 0}</p>
                        </div>
                        <div className="text-right">
                          <p className="text-xs text-neon-cyan">Difficulty</p>
                          <p className="font-bold soft-glow">{block.difficulty}</p>
                        </div>
                        <div className="bg-green-600/20 rounded-lg px-3 py-2">
                          <p className="text-green-400 font-bold">
                            +{formatNumber(block.reward || 0)}
                          </p>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
        {activeTab === 'analytics' && <Analytics chartData={chartData} />}
        {activeTab === 'explorer' && (
          <BlockExplorer
            blocks={blocks}
            formatHash={formatHash}
            formatNumber={formatNumber}
            formatFullNumber={formatFullNumber}
          />
        )}
        {activeTab === 'terminal' && <DiagnosticTerminal nodeUrl={nodeUrl} />}
      </div>
    </div>
  );
};

export default TrinityChainDashboard;
