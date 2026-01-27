import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

const Welcome = () => {
  const navigate = useNavigate();
  const [greeting] = useState(() => {
    const greetings = [
      "Ready to make your wallet cry tears of joy? 😭💰",
      "Time to turn those diamond hands into actual diamonds! 💎👋",
      "Welcome to the exclusive club of crypto degenerates! 🎪🚀",
      "Let's get you richer than a Nigerian prince's email! 👑💸",
      "Ready to HODL like your life depends on it? 🤝💀",
      "Welcome, future crypto millionaire (results not guaranteed)! 🎰💎",
      "Time to make Wall Street bros jealous AF! 📈🔥",
      "Ready to go to the moon? Pack light! 🌙🚀"
    ];
    return greetings[Math.floor(Math.random() * greetings.length)];
  });

  const handleGetStarted = () => {
    navigate('/login');
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-primary-200 via-sage-100 to-amber-100 relative flex items-center justify-center">
      {/* Background pattern */}
      <div className="absolute inset-0 bg-noise opacity-30"></div>

      {/* Main content */}
      <div className="relative z-10 max-w-md w-full mx-4">
        {/* Welcome card */}
        <div className="glass rounded-3xl p-8 text-center animate-fade-up shadow-large">
          {/* Logo/Icon placeholder */}
          <div className="w-20 h-20 mx-auto mb-6 bg-gradient-to-br from-primary-500 to-sage-500 rounded-2xl flex items-center justify-center">
            <svg className="w-10 h-10 text-white" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
            </svg>
          </div>

          {/* Greeting */}
          <h1 className="text-2xl font-bold text-stone-900 mb-2">
            {greeting}
          </h1>

          <p className="text-stone-600 mb-8 leading-relaxed">
            Ready to join the most exclusive crypto community? Let's get you set up with enterprise-grade security and maximum privacy.
          </p>

          {/* Get Started button */}
          <button
            onClick={handleGetStarted}
            className="btn-primary w-full py-4 text-lg font-semibold rounded-xl shadow-medium hover:shadow-large transition-all duration-300 hover:scale-[1.02] active:scale-[0.98]"
          >
            Get Started
          </button>

          {/* Trust indicators */}
          <div className="mt-6 flex items-center justify-center space-x-4 text-xs text-stone-500">
            <span className="flex items-center space-x-1">
              <div className="w-2 h-2 bg-sage-500 rounded-full"></div>
              <span>SOC 2 Certified</span>
            </span>
            <span className="flex items-center space-x-1">
              <div className="w-2 h-2 bg-primary-500 rounded-full"></div>
              <span>Bank-Grade Security</span>
            </span>
          </div>
        </div>

        {/* Bottom text */}
        <p className="text-center text-stone-500 text-sm mt-6">
          Trusted by thousands of crypto professionals worldwide
        </p>
      </div>
    </div>
  );
};

export default Welcome;