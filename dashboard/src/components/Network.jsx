import React from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { Cpu, Network as NetworkIcon, Target, Activity } from 'lucide-react';
import { StatCard } from './StatCard';

const Network = ({ stats, networkData, calculateHashrate, formatNumber }) => {
  const tooltipStyle = {
    backgroundColor: 'rgba(30, 0, 30, 0.8)',
    border: '1px solid #ff00ff',
    borderRadius: '8px',
    color: '#00ffff',
  };

  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-6">
        <StatCard
          icon={<Cpu className="text-neon-cyan" />}
          label="Network Hashrate"
          value={`${calculateHashrate()} H/s`}
          trend="+12.5%"
          color="cyan"
        />
        <StatCard
          icon={<NetworkIcon className="text-neon-cyan" />}
          label="Active Nodes"
          value="1"
          trend="Stable"
          color="blue"
        />
        <StatCard
          icon={<Target className="text-neon-pink" />}
          label="Network Difficulty"
          value={formatNumber(stats?.difficulty || 0)}
          trend={`Target: ${stats?.difficulty || 0}`}
          color="red"
        />
      </div>

      <div className="glass-card neon-border p-6 mb-6">
        <h3 className="text-xl font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
          <Activity />
          Network Performance
        </h3>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={networkData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
            <XAxis dataKey="block" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
            <YAxis yAxisId="left" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
            <YAxis yAxisId="right" orientation="right" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
            <Tooltip contentStyle={tooltipStyle} labelStyle={{ color: '#ffffff' }} />
            <Legend />
            <Line yAxisId="left" type="monotone" dataKey="blockTime" stroke="#00ffff" strokeWidth={2} name="Block Time (s)" />
            <Line yAxisId="right" type="monotone" dataKey="hashrate" stroke="#ff00ff" strokeWidth={2} name="Hashrate" />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </>
  );
};

export default Network;
