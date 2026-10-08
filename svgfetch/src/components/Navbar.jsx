import React from 'react';
import { Terminal, ExternalLink, ShieldCheck, Sparkles } from 'lucide-react';

function GithubIcon({ size = 16 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
      <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017C2 16.446 4.843 20.198 8.796 21.52C9.296 21.611 9.479 21.303 9.479 21.036C9.479 20.8 9.47 20.165 9.466 19.324C6.684 19.932 6.096 17.973 6.096 17.973C5.641 16.815 4.985 16.505 4.985 16.505C4.076 15.882 5.054 15.895 5.054 15.895C6.058 15.966 6.587 16.93 6.587 16.93C7.479 18.47 8.927 18.026 9.497 17.771C9.588 17.118 9.847 16.673 10.133 16.42C7.913 16.166 5.578 15.305 5.578 11.455C5.578 10.358 5.968 9.46 6.608 8.756C6.505 8.5 6.162 7.479 6.706 6.095C6.706 6.095 7.545 5.825 9.458 7.129C10.256 6.906 11.107 6.795 11.954 6.791C12.8 6.795 13.652 6.906 14.452 7.129C16.363 5.825 17.201 6.095 17.201 6.095C17.746 7.479 17.403 8.5 17.3 8.756C17.942 9.46 18.329 10.358 18.329 11.455C18.329 15.317 15.989 16.163 13.761 16.411C14.12 16.721 14.439 17.332 14.439 18.266C14.439 19.605 14.428 20.686 14.428 21.036C14.428 21.306 14.607 21.619 15.118 21.517C19.07 20.192 21.91 16.443 21.91 12.017C21.91 6.484 17.433 2 12 2Z" />
    </svg>
  );
}

export default function Navbar() {
  return (
    <header className="navbar">
      <div className="container nav-content">
        <a href="#" className="nav-brand">
          <div style={{
            width: 32,
            height: 32,
            borderRadius: 8,
            background: 'linear-gradient(135deg, #6366f1, #a855f7)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            boxShadow: '0 2px 8px rgba(99, 102, 241, 0.4)'
          }}>
            <Terminal size={18} strokeWidth={2.5} />
          </div>
          <span>svgfetch</span>
          <span className="brand-badge">v0.2.7</span>
        </a>

        <nav className="nav-links">
          <a href="#playground" className="nav-link">Playground</a>
          <a href="#variants" className="nav-link">Variants</a>
          <a href="#entities" className="nav-link">Entity Matching</a>
          <a href="#commands" className="nav-link">CLI Reference</a>
          <a href="#security" className="nav-link">Security & License</a>
        </nav>

        <div className="nav-actions">
          <a 
            href="https://www.npmjs.com/package/svgfetch" 
            target="_blank" 
            rel="noopener noreferrer"
            className="btn-secondary"
            title="View on npm"
          >
            <span style={{ color: '#cb3837', fontWeight: 800 }}>npm</span>
            <ExternalLink size={14} />
          </a>
          <a 
            href="https://github.com/avdeshjadon/svgfetch" 
            target="_blank" 
            rel="noopener noreferrer"
            className="btn-primary"
            title="Star on GitHub"
          >
            <GithubIcon size={16} />
            <span>GitHub</span>
          </a>
        </div>
      </div>
    </header>
  );
}
