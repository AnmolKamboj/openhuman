import { useNavigate } from 'react-router-dom';
import { openUrl } from '@tauri-apps/plugin-opener';
import { TELEGRAM_BOT_USERNAME } from '../../utils/config';

const Step4GetStarted = () => {
  const navigate = useNavigate();

  const handleOpenTelegram = async () => {
    try {
      await openUrl(`https://t.me/${TELEGRAM_BOT_USERNAME}`);
      // Navigate to home after opening Telegram
      setTimeout(() => {
        navigate('/home');
      }, 1000);
    } catch (error) {
      console.error('Failed to open Telegram:', error);
    }
  };

  const handleSkip = () => {
    navigate('/home');
  };

  return (
    <div className="min-h-screen relative flex items-center justify-center">
      {/* Main content */}
      <div className="relative z-10 max-w-md w-full mx-4">
        {/* Progress indicator */}
        <div className="flex items-center justify-center space-x-2 mb-8">
          <div className="flex items-center">
            <div className="w-8 h-8 bg-primary-500 rounded-full flex items-center justify-center text-white text-sm font-semibold">1</div>
            <div className="w-12 h-1 bg-primary-500 mx-2"></div>
          </div>
          <div className="flex items-center">
            <div className="w-8 h-8 bg-primary-500 rounded-full flex items-center justify-center text-white text-sm font-semibold">2</div>
            <div className="w-12 h-1 bg-primary-500 mx-2"></div>
          </div>
          <div className="flex items-center">
            <div className="w-8 h-8 bg-primary-500 rounded-full flex items-center justify-center text-white text-sm font-semibold">3</div>
            <div className="w-12 h-1 bg-primary-500 mx-2"></div>
          </div>
          <div className="w-8 h-8 bg-primary-500 rounded-full flex items-center justify-center text-white text-sm font-semibold">4</div>
        </div>

        {/* Get Started card */}
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          <div className="text-center mb-8">
            <div className="w-16 h-16 mx-auto mb-4 bg-blue-500 rounded-full flex items-center justify-center">
              <svg className="w-8 h-8 text-white" viewBox="0 0 24 24" fill="currentColor">
                <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
              </svg>
            </div>
            <h1 className="text-2xl font-bold mb-2">
              Get Started
            </h1>
            <p className="opacity-70">
              Start messaging the bot to begin your crypto journey
            </p>
          </div>

          {/* Instructions */}
          <div className="space-y-4 mb-8">
            <div className="bg-stone-800/50 rounded-xl p-6 border border-stone-700">
              <div className="flex items-start space-x-4">
                <div className="w-10 h-10 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
                  <span className="text-white font-bold">1</span>
                </div>
                <div>
                  <h3 className="font-semibold mb-1">Open Telegram</h3>
                  <p className="opacity-70 text-sm">
                    Click the button below to open the AlphaHuman bot in Telegram
                  </p>
                </div>
              </div>
            </div>

            <div className="bg-stone-800/50 rounded-xl p-6 border border-stone-700">
              <div className="flex items-start space-x-4">
                <div className="w-10 h-10 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
                  <span className="text-white font-bold">2</span>
                </div>
                <div>
                  <h3 className="font-semibold mb-1">Start Messaging</h3>
                  <p className="opacity-70 text-sm">
                    Send a message to the bot to get started. Try asking about crypto prices, market trends, or anything crypto-related!
                  </p>
                </div>
              </div>
            </div>

            <div className="bg-stone-800/50 rounded-xl p-6 border border-stone-700">
              <div className="flex items-start space-x-4">
                <div className="w-10 h-10 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
                  <span className="text-white font-bold">3</span>
                </div>
                <div>
                  <h3 className="font-semibold mb-1">Cook! 🔥</h3>
                  <p className="opacity-70 text-sm">
                    The bot will help you with research, analysis, and staying on top of the crypto market
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Open Telegram button */}
          <button
            onClick={handleOpenTelegram}
            className="w-full flex items-center justify-center space-x-3 bg-blue-500 hover:bg-blue-600 active:bg-blue-700 text-white font-semibold py-4 rounded-xl transition-all duration-300 hover:shadow-medium mb-4"
          >
            <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
              <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
            </svg>
            <span>Open Telegram Bot</span>
          </button>

          {/* Skip option */}
          <button
            onClick={handleSkip}
            className="w-full py-3 opacity-60 hover:opacity-100 font-medium transition-opacity"
          >
            Skip for now
          </button>
        </div>

        {/* Back button */}
        <button
          onClick={() => navigate('/onboarding/step3')}
          className="mt-6 w-full opacity-60 hover:opacity-100 text-sm font-medium transition-opacity"
        >
          ← Back
        </button>
      </div>
    </div>
  );
};

export default Step4GetStarted;
