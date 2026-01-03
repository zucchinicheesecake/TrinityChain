import React from 'react';
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { TrendingUp, Activity, Award } from 'lucide-react';

const Analytics = ({ chartData }) => {
  const tooltipStyle = {
    backgroundColor: 'rgba(30, 0, 30, 0.8)',
    border: '1px solid #ff00ff',
    borderRadius: '8px',
    color: '#00ffff',
  };

  return (
    <>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="glass-card neon-border p-6">
          <h3 className="text-xl font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
            <TrendingUp />
            Difficulty Trend
          </h3>
          <ResponsiveContainer width="100%" height={250}>
            <AreaChart data={chartData}>
              <defs>
                <linearGradient id="difficultyGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ff00ff" stopOpacity={0.8} />
                  <stop offset="95%" stopColor="#ff00ff" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
              <XAxis dataKey="block" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
              <YAxis stroke="#00ffff" tick={{ fill: '#00ffff' }} />
              <Tooltip contentStyle={tooltipStyle} labelStyle={{ color: '#ffffff' }} />
              <Area type="monotone" dataKey="difficulty" stroke="#ff00ff" fillOpacity={1} fill="url(#difficultyGradient)" />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div className="glass-card neon-border p-6">
          <h3 className="text-xl font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
            <Activity />
            Transaction Activity
          </h3>
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={chartData}>
              <defs>
                <linearGradient id="txGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#00ffff" stopOpacity={0.8} />
                  <stop offset="95%" stopColor="#00ffff" stopOpacity={0.3} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
              <XAxis dataKey="block" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
              <YAxis stroke="#00ffff" tick={{ fill: '#00ffff' }} />
              <Tooltip contentStyle={tooltipStyle} labelStyle={{ color: '#ffffff' }} />
              <Bar dataKey="transactions" fill="url(#txGradient)" radius={[8, 8, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div className="glass-card neon-border p-6">
        <h3 className="text-xl font-bold mb-4 flex items-center gap-2 soft-glow text-neon-cyan">
          <Award />
          Block Rewards
        </h3>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
            <XAxis dataKey="block" stroke="#00ffff" tick={{ fill: '#00ffff' }} />
            <YAxis stroke="#00ffff" tick={{ fill: '#00ffff' }} />
            <Tooltip contentStyle={tooltipStyle} labelStyle={{ color: '#ffffff' }} />
            <Line type="monotone" dataKey="reward" stroke="#ff00ff" strokeWidth={3} dot={{ fill: '#ff00ff', r: 4 }} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </>
  );
};

export default Analytics;
