import React from 'react';
import { Layers, CheckCircle2, XCircle, ArrowRight, Sparkles, ShieldAlert, Cpu } from 'lucide-react';

export default function VariantsGuide() {
  const variantCards = [
    {
      name: "icon / symbol",
      aliases: ["symbol", "mark"],
      desc: "Fetches isolated app symbols or standalone badges without surrounding logotype text.",
      example: "svgfetch instagram --variant icon"
    },
    {
      name: "wordmark / text",
      aliases: ["text", "logotype"],
      desc: "Fetches the typographic or textual logo without accompanying mascots or badges.",
      example: "svgfetch docker --variant wordmark"
    },
    {
      name: "full / lockup",
      aliases: ["lockup"],
      desc: "Fetches the complete brand presentation containing both mark and wordmark together.",
      example: "svgfetch github --variant full"
    },
    {
      name: "mascot / character",
      aliases: ["character"],
      desc: "Fetches the official brand or OS character illustration rather than the formal logo.",
      example: "svgfetch linux --variant mascot"
    }
  ];

  const entityComparisons = [
    {
      query: "svgfetch facebook",
      resolved: "Facebook Logo (blue circle / F mark)",
      rejected: "Meta Platforms logo is strictly blocked",
      status: "Separated Entities"
    },
    {
      query: "svgfetch linux",
      resolved: "Official Linux Foundation vector banner",
      rejected: "Tux penguin mascot is only chosen if requested as mascot",
      status: "Semantic Precision"
    },
    {
      query: "svgfetch kali linux",
      resolved: "Kali Linux dragon shield logo",
      rejected: "Base Linux or generic kernel graphics are excluded",
      status: "Exact Distribution Match"
    }
  ];

  return (
    <section id="variants" className="section">
      <div className="container">
        <div className="section-header">
          <div className="section-tag">Exact Asset Variants</div>
          <h2 className="section-title heading-display">No More Wrong Logos</h2>
          <p className="section-desc">
            Never download the entire wordmark banner when you just wanted the app icon. Explicit flags and smart natural language parsing give you exactly what you need.
          </p>
        </div>

        {/* Variants Cards Grid */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))', gap: 20, marginBottom: 64 }}>
          {variantCards.map((v, idx) => (
            <div key={idx} className="feature-card">
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
                <span className="brand-variant-badge" style={{ fontSize: '0.8rem', padding: '4px 10px' }}>
                  --variant {v.name.split(' / ')[0]}
                </span>
                <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                  aliases: {v.aliases.join(', ')}
                </span>
              </div>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: 16 }}>
                {v.desc}
              </p>
              <div style={{
                background: 'rgba(0, 0, 0, 0.4)',
                padding: '8px 12px',
                borderRadius: 'var(--radius-sm)',
                fontFamily: 'var(--font-mono)',
                fontSize: '0.78rem',
                color: '#38bdf8'
              }}>
                {v.example}
              </div>
            </div>
          ))}
        </div>

        {/* Entity Matching Section */}
        <div id="entities" style={{
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border-medium)',
          borderRadius: 'var(--radius-xl)',
          padding: 36
        }}>
          <div style={{ maxWidth: 640, marginBottom: 28 }}>
            <span className="section-tag" style={{ color: '#10b981' }}>Entity Resolution Engine</span>
            <h3 className="heading-display" style={{ fontSize: '1.75rem', margin: '8px 0' }}>
              Entity Separation Over Fuzzy Chaos
            </h3>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
              Unlike generic scrapers that confuse parent holding corporations with child products, svgfetch uses curated disambiguation rules.
            </p>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: 16 }}>
            {entityComparisons.map((item, idx) => (
              <div key={idx} style={{
                background: 'var(--bg-tertiary)',
                borderRadius: 'var(--radius-md)',
                padding: 20,
                border: '1px solid var(--border-subtle)'
              }}>
                <div style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.85rem',
                  color: '#fbbf24',
                  marginBottom: 12,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8
                }}>
                  <span>$</span>
                  <span>{item.query}</span>
                </div>

                <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8, marginBottom: 8, fontSize: '0.85rem', color: 'var(--text-primary)' }}>
                  <CheckCircle2 size={16} style={{ color: '#10b981', flexShrink: 0, marginTop: 2 }} />
                  <span>{item.resolved}</span>
                </div>

                <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8, fontSize: '0.85rem', color: 'var(--text-muted)' }}>
                  <XCircle size={16} style={{ color: '#f43f5e', flexShrink: 0, marginTop: 2 }} />
                  <span>{item.rejected}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
