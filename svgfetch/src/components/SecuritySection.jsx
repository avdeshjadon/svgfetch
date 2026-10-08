import React from 'react';
import { Shield, FileCheck, Lock, Scale, Zap, Check } from 'lucide-react';

export default function SecuritySection() {
  const securityFeatures = [
    {
      icon: <Lock size={22} />,
      title: "Zip Slip & Traversal Defense",
      desc: "Every remote filename and archive entry is sanitized against path traversal (../), null-bytes, control characters, and reserved names before writing to disk."
    },
    {
      icon: <FileCheck size={22} />,
      title: "Deterministic Provenance",
      desc: "Downloads record author, source URL, Wikimedia file title, and license inside svgfetch-metadata.json without overwriting previous project provenance."
    },
    {
      icon: <Zap size={22} />,
      title: "Wikimedia API Etiquette",
      desc: "Built-in rate limiter and exponential backoff retry on HTTP 429/5xx errors, respecting Wikimedia Commons community infrastructure."
    },
    {
      icon: <Scale size={22} />,
      title: "GNU AGPLv3 Copyleft",
      desc: "100% Free and Open Source. Anyone can inspect and contribute, while strictly preventing proprietary corporations from locking away your software."
    }
  ];

  return (
    <section id="security" className="section">
      <div className="container">
        <div className="section-header">
          <div className="section-tag" style={{ color: '#10b981' }}>Security & Licensing</div>
          <h2 className="section-title heading-display">Built With Integrity</h2>
          <p className="section-desc">
            Production-grade defensive engineering at every layer — from network requests to local file permissions.
          </p>
        </div>

        <div className="features-grid">
          {securityFeatures.map((feat, idx) => (
            <div key={idx} className="feature-card">
              <div className="feature-icon-wrapper" style={{
                color: idx === 3 ? '#ec4899' : '#6366f1',
                borderColor: idx === 3 ? 'rgba(236, 72, 153, 0.3)' : 'rgba(99, 102, 241, 0.3)',
                background: idx === 3 ? 'rgba(236, 72, 153, 0.1)' : 'rgba(99, 102, 241, 0.1)'
              }}>
                {feat.icon}
              </div>
              <h3 className="feature-title">{feat.title}</h3>
              <p className="feature-desc">{feat.desc}</p>
            </div>
          ))}
        </div>

        {/* License Callout Box */}
        <div style={{
          marginTop: 48,
          background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%)',
          border: '1px solid rgba(99, 102, 241, 0.3)',
          borderRadius: 'var(--radius-lg)',
          padding: 32,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center'
        }}>
          <Scale size={32} style={{ color: '#818cf8', marginBottom: 16 }} />
          <h3 className="heading-display" style={{ fontSize: '1.5rem', marginBottom: 8 }}>
            Licensed under GNU Affero General Public License v3
          </h3>
          <p style={{ color: 'var(--text-secondary)', maxWidth: 680, fontSize: '0.95rem', marginBottom: 20 }}>
            SVGFetch guarantees freedom for all users. Anyone can use, modify, and improve the software for non-commercial or open-source purposes. If any entity distributes modified versions, they must share the source code under the same AGPLv3 terms.
          </p>
          <div style={{ display: 'flex', gap: 12 }}>
            <a 
              href="https://github.com/avdeshjadon/svgfetch/blob/main/LICENSE" 
              target="_blank" 
              rel="noopener noreferrer"
              className="btn-primary"
            >
              Read AGPLv3 License
            </a>
            <a 
              href="https://github.com/avdeshjadon/svgfetch/blob/main/SECURITY.md" 
              target="_blank" 
              rel="noopener noreferrer"
              className="btn-secondary"
            >
              Security Policy
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
