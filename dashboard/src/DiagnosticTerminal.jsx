import React, { useEffect, useState } from 'react';
import { Terminal, X, Maximize2, Minimize2 } from 'lucide-react';

const DiagnosticTerminal = ({ nodeUrl }) => {
  const [logs, setLogs] = useState([]);
  const [isMinimized, setIsMinimized] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [filter, setFilter] = useState('all');
  const logsEndRef = React.useRef(null);

  const scrollToBottom = () => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  };

  useEffect(scrollToBottom, [logs, autoScroll]);

  const addLog = (message, type = 'info', timestamp = new Date()) => {
    setLogs(prev => [...prev, {
      id: Date.now() + Math.random(),
      message,
      type,
      timestamp: timestamp.toLocaleTimeString(),
    }].slice(-100)); // Keep last 100 logs
  };

  useEffect(() => {
    addLog('🚀 Diagnostic Terminal initialized', 'info');
    addLog(`📍 Connected to: ${nodeUrl}`, 'info');

    const interval = setInterval(async () => {
      try {
        const stats = await fetch(`${nodeUrl}/api/blockchain/stats`, { credentials: 'include' }).then(r => r.json());
        addLog(`📦 Blockchain: Height=${stats.height}, Difficulty=${stats.difficulty}`, 'success');
      } catch (e) {
        addLog(`❌ Failed to fetch blockchain stats: ${e.message}`, 'error');
      }
    }, 10000);

    return () => clearInterval(interval);
  }, [nodeUrl]);

  const filteredLogs = filter === 'all' 
    ? logs 
    : logs.filter(log => log.type === filter);

  const getLogColor = (type) => {
    switch (type) {
      case 'success': return '#86efac';
      case 'error': return '#ff6b6b';
      case 'warning': return '#fbbf24';
      case 'info': return '#60a5fa';
      default: return '#e2e8f0';
    }
  };

  if (isMinimized) {
    return (
      <div
        onClick={() => setIsMinimized(false)}
        className="fixed bottom-4 right-4 cursor-pointer glass-card neon-border p-3 hover:bg-dark-purple/70 transition"
      >
        <Terminal size={20} className="text-neon-cyan" />
      </div>
    );
  }

  return (
    <div className={`fixed ${isMaximized ? 'inset-0' : 'bottom-4 right-4 w-96 h-96'} glass-card neon-border flex flex-col z-50 shadow-2xl`}>
      <div className="bg-dark-purple/50 px-4 py-3 border-b border-neon-cyan/20 flex items-center justify-between">
        <div className="flex items-center gap-2 text-neon-cyan">
          <Terminal size={18} />
          <span className="font-bold text-sm soft-glow">Diagnostic Terminal</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setAutoScroll(!autoScroll)}
            className={`p-1 rounded hover:bg-dark-purple transition text-xs ${autoScroll ? 'bg-dark-purple text-green-400' : 'text-slate-400'}`}
            title="Auto-scroll"
          >
            {autoScroll ? '⬇️' : '⏸️'}
          </button>
          <button
            onClick={() => setIsMaximized(!isMaximized)}
            className="p-1 rounded hover:bg-dark-purple transition text-slate-400 hover:text-white"
          >
            {isMaximized ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
          </button>
          <button
            onClick={() => setIsMinimized(true)}
            className="p-1 rounded hover:bg-dark-purple transition text-slate-400 hover:text-white"
          >
            <X size={16} />
          </button>
        </div>
      </div>

      <div className="bg-dark-purple/50 px-4 py-2 border-b border-neon-cyan/20 flex gap-2 text-xs">
        {['all', 'info', 'success', 'warning', 'error'].map(type => (
          <button
            key={type}
            onClick={() => setFilter(type)}
            className={`px-2 py-1 rounded transition glass-card ${
              filter === type
                ? 'bg-neon-pink text-white'
                : 'bg-dark-purple text-neon-cyan hover:bg-dark-purple/70'
            }`}
          >
            {type.charAt(0).toUpperCase() + type.slice(1)}
          </button>
        ))}
        <button
          onClick={() => setLogs([])}
          className="ml-auto px-2 py-1 rounded bg-red-900 text-red-200 hover:bg-red-800 transition text-xs"
        >
          Clear
        </button>
      </div>

      <div className="flex-1 overflow-y-auto bg-dark-purple font-mono text-xs p-3 space-y-1">
        {filteredLogs.length === 0 ? (
          <div className="text-slate-500 text-center py-8">
            {filter === 'all' ? 'No logs yet' : `No ${filter} logs`}
          </div>
        ) : (
          filteredLogs.map(log => (
            <div key={log.id} className="flex gap-2 group hover:bg-dark-purple/70 px-1 py-0.5 rounded transition">
              <span className="text-slate-500 flex-shrink-0">{log.timestamp}</span>
              <span style={{ color: getLogColor(log.type) }} className="font-semibold flex-shrink-0 w-8">
                {log.type === 'info' ? '📘' :
                 log.type === 'success' ? '✅' :
                 log.type === 'warning' ? '⚠️' :
                 log.type === 'error' ? '❌' : '•'}
              </span>
              <span style={{ color: getLogColor(log.type) }} className="flex-1 break-words">
                {log.message}
              </span>
            </div>
          ))
        )}
        <div ref={logsEndRef} />
      </div>

      <div className="bg-dark-purple/50 px-4 py-2 border-t border-neon-cyan/20 text-xs text-neon-cyan">
        Logs: {filteredLogs.length} / {logs.length} | Status: {logs.some(l => l.type === 'success') ? '🟢 Connected' : '🔴 Disconnected'}
      </div>
    </div>
  );
};

export default DiagnosticTerminal;
