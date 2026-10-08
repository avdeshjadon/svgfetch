import React, { useState } from 'react';
import { Terminal as TermIcon, Play, RefreshCw, CheckCircle2 } from 'lucide-react';

export default function TerminalDemo() {
  const [activeScenario, setActiveScenario] = useState('direct');

  const scenarios = {
    direct: {
      name: "Direct Fetch",
      cmd: "svgfetch instagram --variant icon",
      output: [
        { type: "info", text: "✓ Found curated brand match: Instagram (icon)" },
        { type: "text", text: "→ Source: File:Instagram icon.svg on Wikimedia Commons" },
        { type: "text", text: "→ Detected project directory: ./src/assets/" },
        { type: "text", text: "→ Streaming vector content and verifying UTF-8 XML..." },
        { type: "success", text: "✔ Downloaded ./src/assets/Instagram_icon.svg (1.4 KiB)" },
        { type: "success", text: "✔ Updated provenance: ./src/assets/svgfetch-metadata.json" }
      ]
    },
    tui: {
      name: "Interactive TUI",
      cmd: "svgfetch",
      output: [
        { type: "info", text: "┌── SVGFetch Interactive Terminal Explorer ────────────────────────┐" },
        { type: "text", text: "│ Query: docker [3 results]                                        │" },
        { type: "highlight", text: "│ > [1] Docker (container engine) logo.svg  (5.2 KB, CC BY-SA 4.0) │" },
        { type: "text", text: "│   [2] Docker wordmark.svg                 (2.1 KB, Apache-2.0)   │" },
        { type: "text", text: "│   [3] Docker Moby whale mascot.svg        (6.8 KB, CC BY-SA 4.0) │" },
        { type: "info", text: "│ [Preview: V] [Select: Space] [Download: Enter] [Quit: Esc/q]     │" },
        { type: "success", text: "└──────────────────────────────────────────────────────────────────┘" }
      ]
    },
    info: {
      name: "Inspect Info",
      cmd: "svgfetch info docker --variant wordmark",
      output: [
        { type: "info", text: "=== SVGFetch Asset Metadata ===" },
        { type: "text", text: "Brand:        Docker" },
        { type: "text", text: "Variant:      wordmark (resolved via explicit flag)" },
        { type: "text", text: "Title:        File:Docker wordmark.svg" },
        { type: "text", text: "Author:       Docker Inc." },
        { type: "text", text: "License:      Apache-2.0" },
        { type: "text", text: "Direct URL:   https://upload.wikimedia.org/.../Docker_wordmark.svg" },
        { type: "success", text: "Attribution:  Safe for commercial or educational citation" }
      ]
    },
    doctor: {
      name: "CLI Doctor",
      cmd: "svgfetch doctor",
      output: [
        { type: "info", text: "=== SVGFetch Doctor Diagnostics ===" },
        { type: "success", text: "✔ Network Connectivity: Reachable (Wikimedia Commons API 200 OK)" },
        { type: "success", text: "✔ Local Disk Cache: Valid (~/Library/Caches/svgfetch)" },
        { type: "success", text: "✔ Terminal Capabilities: 24-bit TrueColor & Halfblock UTF-8 OK" },
        { type: "success", text: "✔ Write Permissions: Current directory is writable" },
        { type: "success", text: "All checks passed. System ready." }
      ]
    }
  };

  const current = scenarios[activeScenario];

  return (
    <section className="section" style={{ background: 'rgba(15, 20, 34, 0.4)' }}>
      <div className="container">
        <div className="section-header">
          <div className="section-tag">Interactive Terminal</div>
          <h2 className="section-title heading-display">Feels Like Second Nature</h2>
          <p className="section-desc">
            Rebuilt with pure Rust ergonomics. Instant execution, crystal-clear output, and zero background baggage.
          </p>
        </div>

        {/* Scenario Switcher Tabs */}
        <div style={{ display: 'flex', justifyContent: 'center', gap: 10, marginBottom: 24 }}>
          {Object.entries(scenarios).map(([key, item]) => (
            <button
              key={key}
              onClick={() => setActiveScenario(key)}
              className={`variant-chip ${activeScenario === key ? 'active' : ''}`}
            >
              {item.name}
            </button>
          ))}
        </div>

        {/* Terminal Window */}
        <div className="terminal-window">
          <div className="terminal-header">
            <div className="terminal-dots">
              <span className="dot red" />
              <span className="dot yellow" />
              <span className="dot green" />
            </div>
            <div className="terminal-title">svgfetch — terminal session</div>
            <div style={{ width: 44 }} />
          </div>

          <div className="terminal-body">
            <div className="term-line">
              <span className="term-prompt">$</span>
              <span className="term-cmd">{current.cmd}</span>
            </div>

            <div style={{ marginTop: 12 }}>
              {current.output.map((line, idx) => (
                <div 
                  key={idx} 
                  className={`term-output ${
                    line.type === 'success' ? 'term-success' : 
                    line.type === 'info' ? 'term-info' : 
                    line.type === 'highlight' ? 'term-highlight' : ''
                  }`}
                >
                  {line.text}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
