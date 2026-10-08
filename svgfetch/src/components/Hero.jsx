import React, { useState } from 'react';
import { Copy, Check, Terminal, Sparkles, Shield, Cpu, Zap, Box } from 'lucide-react';

export default function Hero() {
  const [activeTab, setActiveTab] = useState('npx');
  const [copied, setCopied] = useState(false);

  const installCommands = {
    npx: 'npx svgfetch',
    npm: 'npm install -g svgfetch',
    cargo: 'cargo install svgfetch --locked',
    brew: 'brew install avdeshjadon/tap/svgfetch',
    curl: 'curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | bash',
    powershell: 'irm https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.ps1 | iex'
  };

  const currentCommand = installCommands[activeTab];

  const handleCopy = () => {
    navigator.clipboard.writeText(currentCommand);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section className="hero-section">
      <div className="container">
        <div className="hero-pill">
          <Sparkles size={14} style={{ color: '#818cf8' }} />
          <span>v0.2.7 Released with Exact Variants & AGPLv3 Protection</span>
        </div>

        <h1 className="hero-title heading-display">
          Discover, Inspect & Download <br />
          <span className="text-gradient">SVGs From Your Terminal</span>
        </h1>

        <p className="hero-subtitle">
          The blazing-fast, pure-Rust CLI tool to fetch official vector assets from Wikimedia Commons straight into your project's assets folder with zero hassle.
        </p>

        {/* The npm-like Install Card */}
        <div className="install-card">
          <div className="install-tabs">
            {Object.keys(installCommands).map((tab) => (
              <button
                key={tab}
                className={`install-tab ${activeTab === tab ? 'active' : ''}`}
                onClick={() => {
                  setActiveTab(tab);
                  setCopied(false);
                }}
              >
                {tab === 'powershell' ? 'PowerShell' : tab}
              </button>
            ))}
          </div>

          <div className="install-cmd-row">
            <div className="install-cmd-code">
              <span className="install-cmd-prefix">$</span>
              <span>{currentCommand}</span>
            </div>
            <button 
              className={`copy-btn ${copied ? 'copied' : ''}`}
              onClick={handleCopy}
              aria-label="Copy install command"
            >
              {copied ? (
                <>
                  <Check size={14} />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy size={14} />
                  <span>Copy</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Quick Highlights Stats */}
        <div className="stats-grid">
          <div className="stat-item">
            <div className="stat-value text-gradient">Rust</div>
            <div className="stat-label">Zero-Cost Memory & Speed</div>
          </div>
          <div className="stat-item">
            <div className="stat-value" style={{ color: '#38bdf8' }}>AGPLv3</div>
            <div className="stat-label">Strict Copyleft License</div>
          </div>
          <div className="stat-item">
            <div className="stat-value" style={{ color: '#10b981' }}>0 Bloat</div>
            <div className="stat-label">Zero Runtime Dependencies</div>
          </div>
          <div className="stat-item">
            <div className="stat-value" style={{ color: '#f59e0b' }}>Cross-OS</div>
            <div className="stat-label">macOS, Linux, Windows</div>
          </div>
        </div>
      </div>
    </section>
  );
}
