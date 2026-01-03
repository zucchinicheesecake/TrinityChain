import React, { useState } from 'react';
import { Search, ChevronDown, ChevronUp, Database, Boxes, Activity } from 'lucide-react';

const DetailRow = ({ label, value }) => (
  <div className="flex justify-between items-center">
    <span className="text-neon-cyan text-sm">{label}</span>
    <span className="font-mono text-sm text-white soft-glow">{value}</span>
  </div>
);

const BlockExplorer = ({ blocks, formatHash, formatNumber, formatFullNumber }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedBlocks, setExpandedBlocks] = useState(new Set());

  const toggleBlockExpansion = (blockIndex) => {
    const newExpanded = new Set(expandedBlocks);
    newExpanded.has(blockIndex) ? newExpanded.delete(blockIndex) : newExpanded.add(blockIndex);
    setExpandedBlocks(newExpanded);
  };

  const filteredBlocks = blocks.filter(
    (block) =>
      !searchQuery ||
      block.hash?.toLowerCase().includes(searchQuery.toLowerCase()) ||
      block.index?.toString().includes(searchQuery) ||
      block.previousHash?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <>
      <div className="glass-card neon-border p-6 mb-6">
        <div className="flex items-center gap-4">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-neon-cyan" size={20} />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search by block height, hash, or previous hash..."
              className="w-full bg-dark-purple border border-neon-pink rounded-lg pl-10 pr-4 py-3 text-white placeholder-neon-pink/50 focus:outline-none focus:border-neon-cyan"
            />
          </div>
          <div className="glass-card px-4 py-3">
            <span className="text-neon-cyan text-sm">{filteredBlocks.length} blocks</span>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        {filteredBlocks.map((block) => (
          <div key={block.index} className="glass-card neon-border overflow-hidden transition-all">
            <div className="p-6 cursor-pointer" onClick={() => toggleBlockExpansion(block.index)}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="bg-gradient-to-br from-neon-pink to-neon-cyan rounded-xl px-4 py-3 shadow-lg">
                    <span className="text-white font-mono font-bold text-xl soft-glow">#{block.index}</span>
                  </div>
                  <div>
                    <p className="text-white font-mono text-sm mb-1">{formatHash(block.hash)}</p>
                    <p className="text-neon-pink text-xs">{new Date(block.timestamp).toLocaleString()}</p>
                  </div>
                </div>
                <div className="flex items-center gap-6">
                  <Stat label="Difficulty" value={block.difficulty} />
                  <Stat label="Nonce" value={block.nonce} />
                  <Stat label="Transactions" value={block.transactions?.length || 0} />
                  <div className="bg-green-600/20 rounded-lg px-4 py-2 border border-green-500/30">
                    <p className="text-green-400 font-bold text-lg">+{formatNumber(block.reward || 0)}</p>
                  </div>
                  <button className="text-neon-cyan hover:text-white transition-colors">
                    {expandedBlocks.has(block.index) ? <ChevronUp size={24} /> : <ChevronDown size={24} />}
                  </button>
                </div>
              </div>
            </div>

            {expandedBlocks.has(block.index) && (
              <div className="border-t border-neon-cyan/20 bg-dark-purple/50 p-6">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <BlockDetails block={block} formatFullNumber={formatFullNumber} />
                  <HashInfo block={block} />
                </div>
                {block.transactions?.length > 0 && <Transactions transactions={block.transactions} formatHash={formatHash} />}
              </div>
            )}
          </div>
        ))}
      </div>
    </>
  );
};

const Stat = ({ label, value }) => (
  <div className="text-center">
    <p className="text-neon-cyan text-xs mb-1">{label}</p>
    <p className="font-bold text-lg soft-glow">{value}</p>
  </div>
);

const BlockDetails = ({ block, formatFullNumber }) => (
  <div>
    <h4 className="text-neon-cyan text-sm font-semibold mb-3 flex items-center gap-2">
      <Database size={16} /> Block Details
    </h4>
    <div className="space-y-3">
      <DetailRow label="Block Height" value={`#${block.index}`} />
      <DetailRow label="Timestamp" value={new Date(block.timestamp).toISOString()} />
      <DetailRow label="Difficulty" value={block.difficulty} />
      <DetailRow label="Nonce" value={block.nonce} />
      <DetailRow label="Reward" value={`${formatFullNumber(block.reward || 0)} TRC`} />
      <DetailRow label="Size" value={`${JSON.stringify(block).length} bytes`} />
    </div>
  </div>
);

const HashInfo = ({ block }) => (
  <div>
    <h4 className="text-neon-cyan text-sm font-semibold mb-3 flex items-center gap-2">
      <Boxes size={16} /> Hash Information
    </h4>
    <div className="space-y-3">
      <HashValue label="Block Hash" value={block.hash} />
      <HashValue label="Previous Hash" value={block.previousHash} />
    </div>
  </div>
);

const HashValue = ({ label, value }) => (
  <div>
    <p className="text-neon-pink text-xs mb-1">{label}</p>
    <p className="font-mono text-xs glass-card p-2 rounded break-all border border-neon-pink/20">{value}</p>
  </div>
);

const Transactions = ({ transactions, formatHash }) => (
  <div className="mt-6">
    <h4 className="text-neon-cyan text-sm font-semibold mb-3 flex items-center gap-2">
      <Activity size={16} /> Transactions ({transactions.length})
    </h4>
    <div className="space-y-2">
      {transactions.map((tx, idx) => (
        <div key={idx} className="glass-card p-3 border border-neon-cyan/10">
          <div className="flex items-center justify-between">
            <p className="font-mono text-sm text-white">{tx.hash ? formatHash(tx.hash) : `Transaction #${idx + 1}`}</p>
            <div className="flex items-center gap-4 text-xs">
              {tx.from && <span className="text-neon-pink">From: {formatHash(tx.from)}</span>}
              {tx.to && <span className="text-neon-cyan">To: {formatHash(tx.to)}</span>}
              {tx.amount && <span className="text-green-400 font-bold">+{tx.amount}</span>}
            </div>
          </div>
        </div>
      ))}
    </div>
  </div>
);

export default BlockExplorer;
