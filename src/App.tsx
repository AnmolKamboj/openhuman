import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-primary-200 via-sage-100 to-amber-100 relative">
      {/* Background pattern for enhanced glass effect */}
      <div className="absolute inset-0 bg-noise opacity-30"></div>
      {/* Header with Glass Effect */}
      <header className="glass border-b border-white/20 safe-area-padding sticky top-0 z-50 relative">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center">
              <h1 className="text-xl font-semibold text-stone-900">
                Crypto Community Platform
              </h1>
            </div>

            {/* Navigation Menu */}
            <nav className="hidden md:flex items-center space-x-1">
              <a href="#" className="nav-item-active">Dashboard</a>
              <a href="#" className="nav-item">Portfolio</a>
              <a href="#" className="nav-item">Chat</a>
              <a href="#" className="nav-item">Markets</a>
            </nav>

            <div className="flex items-center space-x-4">
              <span className="status-online">Connected</span>
            </div>
          </div>
        </div>
      </header>

      {/* Crypto Price Ticker */}
      <div className="glass border-b border-white/10 py-3 relative overflow-hidden">
        <div className="animate-ticker whitespace-nowrap">
          <span className="inline-flex items-center space-x-8 font-mono text-sm">
            <span className="flex items-center space-x-2">
              <span style={{color: '#F7931A'}} className="font-semibold">BTC</span>
              <span className="price-positive">$45,250.00 +2.5%</span>
            </span>
            <span className="flex items-center space-x-2">
              <span style={{color: '#627EEA'}} className="font-semibold">ETH</span>
              <span className="price-negative">$3,120.50 -1.8%</span>
            </span>
            <span className="flex items-center space-x-2">
              <span className="text-stone-700 font-semibold">ADA</span>
              <span className="price-neutral">$0.52 0.0%</span>
            </span>
            <span className="flex items-center space-x-2">
              <span className="text-stone-700 font-semibold">SOL</span>
              <span className="price-positive">$98.75 +4.2%</span>
            </span>
            <span className="flex items-center space-x-2">
              <span className="text-stone-700 font-semibold">USDC</span>
              <span className="price-neutral" style={{color: '#5B9BF3'}}>$1.00 0.0%</span>
            </span>
          </span>
        </div>
      </div>

      {/* Main Content */}
      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 relative">
        {/* Welcome Section with Glass Effect */}
        <div className="glass rounded-xl p-8 mb-8 animate-fade-in-up shadow-large">
          <div className="text-center">
            <h2 className="text-2xl font-semibold text-neutral-900 mb-4">
              Welcome to Your Crypto Community Hub
            </h2>
            <p className="text-neutral-600 mb-6">
              Built with Tauri + React, designed for traders, investors, and crypto enthusiasts.
              This platform prioritizes trust, security, and seamless communication.
            </p>

            {/* Demo Interaction */}
            <div className="max-w-md mx-auto">
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  greet();
                }}
                className="space-y-4"
              >
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Enter your name to test the connection..."
                  className="input-primary"
                />
                <button
                  type="submit"
                  className="btn-primary w-full"
                  disabled={!name.trim()}
                >
                  Test Connection
                </button>
              </form>

              {greetMsg && (
                <div className="mt-4 p-4 bg-success-50 border border-success-200 rounded-lg animate-fade-in-up">
                  <p className="text-success-800 font-medium">{greetMsg}</p>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Feature Preview Cards */}
        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
          {/* Real-time Messaging */}
          <div className="glass rounded-xl p-6 hover:shadow-medium transition-all duration-300 hover:scale-105">
            <div className="flex items-center mb-3">
              <div className="w-8 h-8 bg-primary-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-4 h-4 text-primary-600" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M18 10c0 3.866-3.582 7-8 7a8.841 8.841 0 01-4.083-.98L2 17l1.338-3.123C2.493 12.767 2 11.434 2 10c0-3.866 3.582-7 8-7s8 3.134 8 7zM7 9H5v2h2V9zm8 0h-2v2h2V9zM9 9h2v2H9V9z" clipRule="evenodd" />
                </svg>
              </div>
              <h3 className="font-semibold text-neutral-900">Real-time Chat</h3>
            </div>
            <p className="text-neutral-600 text-sm">
              Secure messaging for crypto discussions with end-to-end encryption and group channels.
            </p>
          </div>

          {/* Portfolio Integration */}
          <div className="glass rounded-xl p-6 hover:shadow-medium transition-all duration-300 hover:scale-105">
            <div className="flex items-center mb-3">
              <div className="w-8 h-8 bg-success-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-4 h-4 text-success-600" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M2 11a1 1 0 011-1h2a1 1 0 011 1v5a1 1 0 01-1 1H3a1 1 0 01-1-1v-5zM8 7a1 1 0 011-1h2a1 1 0 011 1v9a1 1 0 01-1 1H9a1 1 0 01-1-1V7zM14 4a1 1 0 011-1h2a1 1 0 011 1v12a1 1 0 01-1 1h-2a1 1 0 01-1-1V4z" />
                </svg>
              </div>
              <h3 className="font-semibold text-neutral-900">Portfolio Tracking</h3>
            </div>
            <p className="text-neutral-600 text-sm">
              Monitor your crypto investments with real-time price updates and performance analytics.
            </p>
          </div>

          {/* Community Features */}
          <div className="glass rounded-xl p-6 hover:shadow-medium transition-all duration-300 hover:scale-105">
            <div className="flex items-center mb-3">
              <div className="w-8 h-8 bg-warning-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-4 h-4 text-warning-600" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3z" />
                </svg>
              </div>
              <h3 className="font-semibold text-neutral-900">Expert Network</h3>
            </div>
            <p className="text-neutral-600 text-sm">
              Connect with traders, researchers, and KOLs in specialized crypto communities.
            </p>
          </div>
        </div>

        {/* Chat Messages Demo */}
        <div className="glass rounded-xl p-8 mb-8 shadow-medium">
          <h3 className="text-lg font-semibold text-stone-900 mb-6">
            💬 Live Chat Demo
          </h3>

          <div className="space-y-4 mb-6 max-h-64 overflow-y-auto scrollbar-thin">
            <div className="flex justify-start">
              <div className="message-received">
                Hey everyone! BTC looking bullish 🚀
              </div>
            </div>
            <div className="flex justify-end">
              <div className="message-sent">
                Agreed! Just bought more at $45k
              </div>
            </div>
            <div className="flex justify-start">
              <div className="message-received">
                <span className="font-mono text-xs opacity-75">0x1a2b3c...d4e5f6</span><br/>
                Anyone seeing this DeFi yield opportunity?
              </div>
            </div>
            <div className="flex justify-end">
              <div className="message-sent">
                Which protocol? Share the alpha! 👀
              </div>
            </div>
          </div>

          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Type your message..."
              className="input-primary flex-1"
            />
            <button className="btn-primary px-6">Send</button>
          </div>
        </div>

        {/* Design System Showcase */}
        <div className="grid md:grid-cols-2 gap-8 mb-8">
          {/* Buttons & Forms */}
          <div className="glass rounded-xl p-6 shadow-medium">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">
              🎛️ Interactive Components
            </h3>

            <div className="space-y-6">
              {/* Button Examples */}
              <div>
                <h4 className="text-sm font-medium text-stone-700 mb-3">Action Buttons</h4>
                <div className="grid grid-cols-2 gap-3">
                  <button className="btn-primary">Buy Crypto</button>
                  <button className="btn-secondary">View Details</button>
                  <button className="btn-success">Confirm Trade</button>
                  <button className="btn-danger">Cancel Order</button>
                </div>
              </div>

              {/* Form Example */}
              <div>
                <h4 className="text-sm font-medium text-stone-700 mb-3">Trading Form</h4>
                <div className="space-y-3">
                  <input
                    type="text"
                    placeholder="Enter amount (BTC)"
                    className="input-primary font-mono"
                  />
                  <select className="input-primary">
                    <option>Market Order</option>
                    <option>Limit Order</option>
                    <option>Stop Loss</option>
                  </select>
                </div>
              </div>
            </div>
          </div>

          {/* Status & Loading States */}
          <div className="glass rounded-xl p-6 shadow-medium">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">
              📊 Status Indicators
            </h3>

            <div className="space-y-6">
              {/* Status Examples */}
              <div>
                <h4 className="text-sm font-medium text-stone-700 mb-3">Connection Status</h4>
                <div className="flex flex-wrap gap-3">
                  <span className="status-online">Trading Active</span>
                  <span className="status-offline">Market Closed</span>
                  <span className="status-warning">Order Pending</span>
                </div>
              </div>

              {/* Loading States */}
              <div>
                <h4 className="text-sm font-medium text-stone-700 mb-3">Loading States</h4>
                <div className="space-y-2">
                  <div className="loading-pulse h-4 w-3/4"></div>
                  <div className="loading-pulse h-4 w-1/2"></div>
                  <div className="loading-pulse h-4 w-2/3"></div>
                </div>
              </div>

              {/* Crypto Addresses */}
              <div>
                <h4 className="text-sm font-medium text-stone-700 mb-3">Wallet Address</h4>
                <div className="bg-stone-100 rounded-lg p-3 font-mono text-xs break-all border">
                  bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="mt-16 glass border-t border-white/20">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="text-center text-sm text-neutral-500">
            <p>Built with Tauri 2.x, React 19, TypeScript, and Tailwind CSS</p>
            <p className="mt-1">Designed for trust, security, and seamless crypto community interactions</p>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;