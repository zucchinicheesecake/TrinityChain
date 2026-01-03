import React from 'react';
import { RefreshCw, Settings, Play, Pause } from 'lucide-react';

const Header = ({
  autoRefresh,
  setAutoRefresh,
  fetchData,
  loading,
  showSettings,
  setShowSettings,
  error,
  nodeUrl,
  setNodeUrl,
  refreshInterval,
  setRefreshInterval,
}) => {
  return (
    <div className="glass-card neon-border sticky top-0 z-50 mb-6">
      <div className="max-w-7xl mx-auto px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-3">
              <div className="relative">
                <img src="/logo.png" alt="TrinityChain" className="w-10 h-10 object-contain" />
                <div className="absolute -top-1 -right-1 w-3 h-3 bg-green-500 rounded-full border-2 border-dark-purple animate-pulse"></div>
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white soft-glow">
                  TrinityChain
                </h1>
                <p className="text-xs text-neon-cyan">Chain Dashboard v0.2.0</p>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={() => setAutoRefresh(!autoRefresh)}
              className={`p-2 rounded-lg transition-all glass-card ${
                autoRefresh ? 'bg-green-600 hover:bg-green-700' : 'bg-slate-700 hover:bg-slate-600'
              }`}
              title={autoRefresh ? 'Pause auto-refresh' : 'Resume auto-refresh'}
            >
              {autoRefresh ? <Pause size={18} /> : <Play size={18} />}
            </button>
            <button
              onClick={fetchData}
              className="p-2 rounded-lg bg-purple-600 hover:bg-purple-700 transition-all glass-card"
              title="Manual Refresh"
            >
              <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
            </button>
            <button
              onClick={() => setShowSettings(!showSettings)}
              className="p-2 rounded-lg bg-slate-700 hover:bg-slate-600 transition-all glass-card"
              title="Settings"
            >
              <Settings size={18} />
            </button>
            <div className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg glass-card">
              <div
                className={`w-2 h-2 rounded-full ${
                  error ? 'bg-red-500' : 'bg-green-500'
                } animate-pulse`}
              ></div>
              <span className="text-xs text-neon-cyan">{error ? 'Offline' : 'Live'}</span>
            </div>
          </div>
        </div>

        {showSettings && (
          <div className="mt-4 p-4 glass-card">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="text-sm text-neon-cyan mb-2 block">Node URL</label>
                <input
                  type="text"
                  value={nodeUrl}
                  onChange={(e) => setNodeUrl(e.target.value)}
                  className="w-full bg-dark-purple border border-neon-pink rounded px-4 py-2 text-white focus:outline-none focus:border-neon-cyan"
                  placeholder="http://localhost:3000"
                />
              </div>
              <div>
                <label className="text-sm text-neon-cyan mb-2 block">
                  Refresh Interval (ms)
                </label>
                <input
                  type="number"
                  value={refreshInterval}
                  onChange={(e) => setRefreshInterval(parseInt(e.target.value) || 3000)}
                  className="w-full bg-dark-purple border border-neon-pink rounded px-4 py-2 text-white focus:outline-none focus:border-neon-cyan"
                  min="1000"
                  step="1000"
                />
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default Header;
