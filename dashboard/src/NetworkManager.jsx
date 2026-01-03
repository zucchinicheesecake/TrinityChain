import React, { useState, useEffect } from 'react';
import { Network, Server, Globe, Zap, Activity, TrendingUp, AlertCircle, RefreshCw } from 'lucide-react';

const NetworkManager = ({ nodeUrl }) => {
  const [networkInfo, setNetworkInfo] = useState(null);
  const [peers, setPeers] = useState([]);
  const [blockchainStats, setBlockchainStats] = useState(null);
  const [apiStats, setApiStats] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    fetchNetworkData();
    const interval = setInterval(fetchNetworkData, 5000);
    return () => clearInterval(interval);
  }, [nodeUrl]);

  const fetchNetworkData = async () => {
    setLoading(true);
    try {
      const [infoRes, peersRes, statsRes, apiRes] = await Promise.all([
        fetch(`${nodeUrl}/api/network/info`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/api/network/peers`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/api/blockchain/stats`, { credentials: 'include' }).catch(() => ({ ok: false })),
        fetch(`${nodeUrl}/stats`, { credentials: 'include' }).catch(() => ({ ok: false }))
      ]);

      if (infoRes.ok) setNetworkInfo(await infoRes.json());
      if (peersRes.ok) setPeers((await peersRes.json()).peers || []);
      if (statsRes.ok) setBlockchainStats(await statsRes.json());
      if (apiRes.ok) setApiStats(await apiRes.json());

      setError('');
    } catch (err) {
      setError('Failed to fetch network data');
    } finally {
      setLoading(false);
    }
  };

  const formatNumber = (num) => (num >= 1_000_000 ? `${(num / 1_000_000).toFixed(2)}M` : num >= 1000 ? `${(num / 1000).toFixed(2)}K` : String(num));
  const formatUptime = (seconds) => {
    const d = Math.floor(seconds / 86400);
    const h = Math.floor((seconds % 86400) / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return d > 0 ? `${d}d ${h}h` : h > 0 ? `${h}h ${m}m` : `${m}m`;
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold flex items-center gap-2 soft-glow text-neon-cyan">
          <Globe size={28} />
          Network Status
        </h2>
        <button
          onClick={fetchNetworkData}
          disabled={loading}
          className="p-2 glass-card neon-border rounded-lg transition-all disabled:opacity-50"
          title="Refresh"
        >
          <RefreshCw size={20} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {error && (
        <div className="flex items-center gap-3 p-4 rounded-lg glass-card neon-border border-red-500 text-red-200">
          <AlertCircle size={20} />
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <StatBox icon={<Server className="text-blue-400" />} label="Connected Peers" value={peers.length} />
        <StatBox icon={<Zap className="text-purple-400" />} label="Chain Height" value={blockchainStats?.height || 0} />
        <StatBox icon={<TrendingUp className="text-orange-400" />} label="Difficulty" value={blockchainStats?.difficulty || 0} />
        <StatBox icon={<Activity className="text-green-400" />} label="Mempool" value={blockchainStats?.mempool_size || 0} />
      </div>

      {apiStats && <ApiStats stats={apiStats} formatNumber={formatNumber} formatUptime={formatUptime} />}

      <div className="glass-card neon-border p-6">
        <h3 className="text-lg font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
          <Network size={24} />
          Connected Peers ({peers.length})
        </h3>
        {peers.length === 0 ? (
          <div className="text-center p-8">
            <Server size={48} className="mx-auto text-slate-400 mb-4 opacity-50" />
            <p className="text-neon-pink">No peers connected</p>
          </div>
        ) : (
          <div className="space-y-3">
            {peers.map((peer, idx) => <Peer key={idx} peer={peer} idx={idx} />)}
          </div>
        )}
      </div>

      <div className="glass-card neon-border p-6">
        <h3 className="text-lg font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
          <Globe size={24} />
          Node Information
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <InfoBox label="Node URL" value={nodeUrl} />
          <InfoBox label="Network ID" value="TrinityChain" />
          <InfoBox label="Protocol Version" value={networkInfo?.protocol_version || '1.0.0'} />
          <InfoBox label="Sync Status" value="Synchronized" success />
          <InfoBox label="Connection Status" value="Connected" success />
          <InfoBox label="Last Update" value={new Date().toLocaleTimeString()} />
        </div>
      </div>

      {peers.length === 0 && (
        <div className="glass-card neon-border border-yellow-500/30 p-4">
          <div className="flex gap-3">
            <AlertCircle className="text-yellow-400 flex-shrink-0" size={20} />
            <div className="text-sm text-yellow-200">
              <p className="font-semibold mb-1">No Peers Connected</p>
              <p>The node is running but not connected to other peers.</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

const StatBox = ({ icon, label, value }) => (
  <div className="glass-card neon-border p-6">
    <div className="flex items-center justify-between mb-2">
      <p className="text-neon-cyan text-sm">{label}</p>
      {icon}
    </div>
    <p className="text-4xl font-bold soft-glow">{value}</p>
  </div>
);

const ApiStats = ({ stats, formatNumber, formatUptime }) => (
  <div className="glass-card neon-border p-6">
    <h3 className="text-lg font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
      <Activity size={24} />
      API Statistics
    </h3>
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      <ApiStatBox label="Total Requests" value={formatNumber(stats.total_requests)} />
      <ApiStatBox label="Success Rate" value={`${(stats.successful_requests / stats.total_requests * 100 || 0).toFixed(1)}%`} color="text-green-400" />
      <ApiStatBox label="Uptime" value={formatUptime(stats.uptime_seconds)} />
      <ApiStatBox label="Blocks Mined" value={stats.blocks_mined} color="text-green-400" />
    </div>
  </div>
);

const ApiStatBox = ({ label, value, color = '' }) => (
  <div className="glass-card p-4">
    <p className="text-neon-cyan text-xs mb-1">{label}</p>
    <p className={`text-2xl font-bold soft-glow ${color}`}>{value}</p>
  </div>
);

const Peer = ({ peer, idx }) => (
  <div className="glass-card p-4 hover:bg-dark-purple/70 transition-all border border-neon-cyan/10 hover:border-neon-cyan/30">
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-3">
        <div className="bg-neon-cyan/20 rounded-lg p-2">
          <Server className="text-neon-cyan" size={20} />
        </div>
        <div>
          <p className="text-white font-semibold font-mono text-sm">{peer.address || `Peer #${idx + 1}`}</p>
          <p className="text-neon-pink text-xs">Connected</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
        <span className="text-green-400 text-xs font-semibold">ACTIVE</span>
      </div>
    </div>
  </div>
);

const InfoBox = ({ label, value, success }) => (
  <div className="glass-card p-4">
    <p className="text-neon-cyan text-xs mb-1">{label}</p>
    {success ? (
      <div className="flex items-center gap-2">
        <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
        <p className="font-semibold text-white">{value}</p>
      </div>
    ) : (
      <p className="font-mono text-sm text-white break-all">{value}</p>
    )}
  </div>
);

export default NetworkManager;
