import React from 'react';
import Navbar from './components/Navbar';
import Hero from './components/Hero';
import Playground from './components/Playground';
import TerminalDemo from './components/TerminalDemo';
import VariantsGuide from './components/VariantsGuide';
import CommandsReference from './components/CommandsReference';
import SecuritySection from './components/SecuritySection';
import Footer from './components/Footer';

export default function App() {
  return (
    <div className="app-container">
      <Navbar />
      <main>
        <Hero />
        <Playground />
        <TerminalDemo />
        <VariantsGuide />
        <CommandsReference />
        <SecuritySection />
      </main>
      <Footer />
    </div>
  );
}
