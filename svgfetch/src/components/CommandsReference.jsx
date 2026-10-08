import React, { useState } from 'react';
import { CLI_COMMANDS } from '../data/brandsData';
import { Copy, Check, Terminal, HelpCircle } from 'lucide-react';

export default function CommandsReference() {
  const [copiedCmd, setCopiedCmd] = useState(null);

  const handleCopy = (text) => {
    navigator.clipboard.writeText(text);
    setCopiedCmd(text);
    setTimeout(() => setCopiedCmd(null), 2000);
  };

  const flags = [
    { flag: "--variant <VARIANT>", desc: "Request exact variant: default, icon, wordmark, full, or mascot." },
    { flag: "-o, --out <DIR>", desc: "Target download directory (defaults to auto-detected project assets)." },
    { flag: "--dry-run", desc: "Resolve entity and simulate download without writing files to disk." },
    { flag: "--zip", desc: "Package downloaded vectors into a clean ZIP archive." },
    { flag: "--offline", desc: "Operate purely from local cache without making network requests." },
    { flag: "--no-cache", desc: "Bypass HTTP cache and fetch the newest vector from Wikimedia." },
    { flag: "-v, --verbose", desc: "Enable detailed debug logs and HTTP request tracing." },
    { flag: "--json", desc: "Output machine-readable JSON for integration into scripts or CI." }
  ];

  return (
    <section id="commands" className="section" style={{ background: 'rgba(15, 20, 34, 0.5)' }}>
      <div className="container">
        <div className="section-header">
          <div className="section-tag">CLI Command Matrix</div>
          <h2 className="section-title heading-display">Complete Command Reference</h2>
          <p className="section-desc">
            Clean, composable subcommands designed for interactive developers as well as automated CI pipelines.
          </p>
        </div>

        {/* Commands Table */}
        <div className="cmd-table-wrapper" style={{ marginBottom: 48 }}>
          <table className="cmd-table">
            <thead>
              <tr>
                <th>Command & Example</th>
                <th>Aliases</th>
                <th>Description</th>
                <th style={{ textAlign: 'right' }}>Copy</th>
              </tr>
            </thead>
            <tbody>
              {CLI_COMMANDS.map((item, idx) => (
                <tr key={idx}>
                  <td>
                    <div style={{ fontWeight: 600, color: '#f8fafc', marginBottom: 4 }}>
                      {item.command}
                    </div>
                    <div style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: '0.78rem',
                      color: '#38bdf8'
                    }}>
                      $ {item.example}
                    </div>
                  </td>
                  <td>
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                      {item.alias}
                    </span>
                  </td>
                  <td style={{ color: 'var(--text-secondary)' }}>
                    {item.description}
                  </td>
                  <td style={{ textAlign: 'right' }}>
                    <button
                      className={`copy-btn ${copiedCmd === item.example ? 'copied' : ''}`}
                      onClick={() => handleCopy(item.example)}
                      aria-label="Copy command"
                    >
                      {copiedCmd === item.example ? <Check size={14} /> : <Copy size={14} />}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Global Flags Matrix */}
        <div style={{
          background: 'var(--bg-card)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-lg)',
          padding: 32
        }}>
          <h3 className="heading-display" style={{ fontSize: '1.25rem', marginBottom: 20 }}>
            Global Options & Flags
          </h3>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 16 }}>
            {flags.map((f, idx) => (
              <div key={idx} style={{
                background: 'var(--bg-tertiary)',
                padding: 16,
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--border-subtle)'
              }}>
                <div style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.825rem',
                  fontWeight: 600,
                  color: '#a5b4fc',
                  marginBottom: 6
                }}>
                  {f.flag}
                </div>
                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                  {f.desc}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
