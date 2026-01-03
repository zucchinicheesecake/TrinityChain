import React from 'react';

export const HeroStatCard = ({ icon, label, value, subtext, gradient }) => (
  <div
    className={`glass-card neon-border rounded-2xl p-6 shadow-xl transform hover:scale-105 transition-all`}
  >
    <div className="flex items-start justify-between mb-3">
      <div className="bg-white/10 backdrop-blur-sm rounded-xl p-3">{icon}</div>
    </div>
    <p className="text-neon-cyan text-sm mb-1">{label}</p>
    <p className="text-3xl font-bold text-white mb-1 soft-glow">{value}</p>
    <p className="text-white/60 text-xs">{subtext}</p>
  </div>
);

export const StatCard = ({ icon, label, value, trend, color }) => {
  const colorClasses = {
    cyan: 'border-neon-cyan/20 hover:border-neon-cyan/40',
    blue: 'border-blue-500/20 hover:border-blue-500/40',
    red: 'border-red-500/20 hover:border-red-500/40',
  };

  return (
    <div
      className={`glass-card neon-border rounded-2xl p-6 shadow-xl transition-all hover:scale-105`}
    >
      <div className="flex items-center gap-3 mb-4">
        <div className={`bg-${color}-600/20 rounded-lg p-3`}>{icon}</div>
        <span className="text-neon-cyan text-sm font-medium">{label}</span>
      </div>
      <div className="text-4xl font-bold mb-2 soft-glow">{value}</div>
      <div className="text-sm text-neon-pink">{trend}</div>
    </div>
  );
};
