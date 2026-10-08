import React, { useState } from 'react';
import { BRANDS_DATA } from '../data/brandsData';
import { Search, Copy, Check, Terminal, Code, Eye, X, Filter } from 'lucide-react';

export default function Playground() {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedVariant, setSelectedVariant] = useState('all');
  const [copiedId, setCopiedId] = useState(null);
  const [previewModal, setPreviewModal] = useState(null);

  const variants = [
    { label: 'All Variants', value: 'all' },
    { label: 'Icon / Symbol', value: 'icon' },
    { label: 'Wordmark / Text', value: 'wordmark' },
    { label: 'Full Lockup', value: 'full' },
    { label: 'Mascot', value: 'mascot' }
  ];

  const filteredBrands = BRANDS_DATA.filter((item) => {
    const matchesSearch = item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          item.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          item.category.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesVariant = selectedVariant === 'all' || item.variant === selectedVariant;
    return matchesSearch && matchesVariant;
  });

  const generatedCliCmd = searchQuery.trim() 
    ? `svgfetch ${searchQuery.trim().toLowerCase()}${selectedVariant !== 'all' ? ` --variant ${selectedVariant}` : ''}`
    : `svgfetch instagram${selectedVariant !== 'all' ? ` --variant ${selectedVariant}` : ''}`;

  const copyToClipboard = (text, id) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <section id="playground" className="section">
      <div className="container">
        <div className="section-header">
          <div className="section-tag">Interactive Demo</div>
          <h2 className="section-title heading-display">Test svgfetch In Your Browser</h2>
          <p className="section-desc">
            Search popular brands, toggle exact variants, and copy the CLI command or raw SVG code directly.
          </p>
        </div>

        <div className="playground-card">
          {/* Controls: Search & Variant Chips */}
          <div className="playground-controls">
            <div className="search-input-wrapper">
              <Search className="search-input-icon" size={18} />
              <input
                type="text"
                className="search-input"
                placeholder="Search brands (e.g. instagram, docker, meta, tux)..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>

            <div className="variant-chips">
              {variants.map((v) => (
                <button
                  key={v.value}
                  className={`variant-chip ${selectedVariant === v.value ? 'active' : ''}`}
                  onClick={() => setSelectedVariant(v.value)}
                >
                  {v.label}
                </button>
              ))}
            </div>
          </div>

          {/* Dynamic CLI Command Bar */}
          <div className="cli-preview-bar">
            <div className="cli-preview-code">
              <Terminal size={16} style={{ color: '#818cf8' }} />
              <span>{generatedCliCmd}</span>
            </div>
            <button
              className={`copy-btn ${copiedId === 'cli-main' ? 'copied' : ''}`}
              onClick={() => copyToClipboard(generatedCliCmd, 'cli-main')}
            >
              {copiedId === 'cli-main' ? <Check size={14} /> : <Copy size={14} />}
              <span>{copiedId === 'cli-main' ? 'Copied' : 'Copy CLI'}</span>
            </button>
          </div>

          {/* Rendered SVG Brands Grid */}
          <div className="brands-grid">
            {filteredBrands.length > 0 ? (
              filteredBrands.map((brand) => (
                <div key={brand.id} className="brand-card">
                  <div 
                    className="svg-preview-box"
                    dangerouslySetInnerHTML={{ __html: brand.svgContent }}
                    onClick={() => setPreviewModal(brand)}
                    style={{ cursor: 'pointer' }}
                    title="Click to inspect full vector"
                  />
                  <h3 className="brand-name">{brand.name}</h3>
                  <span className="brand-variant-badge">variant: {brand.variant}</span>

                  <div className="brand-actions">
                    <button
                      className="brand-action-btn"
                      onClick={() => copyToClipboard(brand.cliCommand, `cmd-${brand.id}`)}
                      title="Copy CLI command"
                    >
                      {copiedId === `cmd-${brand.id}` ? <Check size={13} /> : <Terminal size={13} />}
                      <span>{copiedId === `cmd-${brand.id}` ? 'Copied' : 'Command'}</span>
                    </button>
                    <button
                      className="brand-action-btn"
                      onClick={() => copyToClipboard(brand.svgContent, `svg-${brand.id}`)}
                      title="Copy raw SVG markup"
                    >
                      {copiedId === `svg-${brand.id}` ? <Check size={13} /> : <Code size={13} />}
                      <span>{copiedId === `svg-${brand.id}` ? 'Copied' : 'SVG'}</span>
                    </button>
                    <button
                      className="brand-action-btn"
                      onClick={() => setPreviewModal(brand)}
                      title="Inspect preview"
                    >
                      <Eye size={13} />
                    </button>
                  </div>
                </div>
              ))
            ) : (
              <div style={{ gridColumn: '1 / -1', textAlign: 'center', padding: '48px 0', color: 'var(--text-muted)' }}>
                No assets matched your search query. Try another brand name or reset the variant filter.
              </div>
            )}
          </div>
        </div>

        {/* Modal for Full Vector Inspection */}
        {previewModal && (
          <div style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0, 0, 0, 0.8)',
            backdropFilter: 'blur(8px)',
            zIndex: 1000,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 24
          }} onClick={() => setPreviewModal(null)}>
            <div style={{
              background: '#0d1117',
              border: '1px solid var(--border-medium)',
              borderRadius: 'var(--radius-lg)',
              maxWidth: 520,
              width: '100%',
              padding: 32,
              position: 'relative'
            }} onClick={(e) => e.stopPropagation()}>
              <button 
                onClick={() => setPreviewModal(null)}
                style={{
                  position: 'absolute',
                  top: 16,
                  right: 16,
                  color: 'var(--text-muted)',
                  padding: 4
                }}
              >
                <X size={20} />
              </button>

              <h3 className="heading-display" style={{ marginBottom: 4 }}>{previewModal.name}</h3>
              <p style={{ color: 'var(--text-muted)', fontSize: '0.85rem', marginBottom: 20 }}>
                {previewModal.fileName}
              </p>

              <div style={{
                background: 'rgba(255, 255, 255, 0.04)',
                borderRadius: 'var(--radius-md)',
                padding: 40,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: 20
              }}>
                <div 
                  style={{ width: 140, height: 140 }}
                  dangerouslySetInnerHTML={{ __html: previewModal.svgContent }}
                />
              </div>

              <div style={{ display: 'flex', gap: 12 }}>
                <button
                  className="btn-primary"
                  style={{ flex: 1, justifyContent: 'center' }}
                  onClick={() => copyToClipboard(previewModal.cliCommand, 'modal-cmd')}
                >
                  {copiedId === 'modal-cmd' ? <Check size={16} /> : <Terminal size={16} />}
                  <span>{copiedId === 'modal-cmd' ? 'Copied Command' : 'Copy CLI Command'}</span>
                </button>
                <button
                  className="btn-secondary"
                  style={{ flex: 1, justifyContent: 'center' }}
                  onClick={() => copyToClipboard(previewModal.svgContent, 'modal-svg')}
                >
                  {copiedId === 'modal-svg' ? <Check size={16} /> : <Code size={16} />}
                  <span>{copiedId === 'modal-svg' ? 'Copied SVG' : 'Copy Raw SVG'}</span>
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
