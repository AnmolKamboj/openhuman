import { openUrl } from '@tauri-apps/plugin-opener';
import { TELEGRAM_BOT_USERNAME } from '../../../utils/config';
import ConnectionIndicator from '../../../components/ConnectionIndicator';

interface GetStartedStepProps {
  onComplete: () => void;
}

const GetStartedStep = ({ onComplete }: GetStartedStepProps) => {
  const handleOpenTelegram = async () => {
    try {
      // Open Telegram and navigate to home immediately
      await openUrl(`https://t.me/${TELEGRAM_BOT_USERNAME}`);
      onComplete();
    } catch (error) {
      console.error('Failed to open Telegram:', error);
      // Still navigate to home even if opening Telegram fails
      onComplete();
    }
  };

  return (
    <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
      <div className="text-center mb-4">
        <h1 className="text-xl font-bold mb-2">You Are Ready, Soldier!</h1>
        <p className="opacity-70 text-sm">
          Alright you're all set up, just message your assistant and you're ready to cook!
        </p>
      </div>

      <ConnectionIndicator
        description="Your browser is now connected to the AlphaHuman AI Models. Keep this tab open to keep the connection alive."
      />

      <div className="space-y-3 mb-4">
        <div className="bg-stone-800/50 rounded-xl p-4 border border-stone-700">
          <div className="flex items-start space-x-3">
            <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
              <span className="text-white font-bold text-xs">1</span>
            </div>
            <div>
              <h3 className="font-semibold mb-1 text-sm">Open Telegram</h3>
              <p className="opacity-70 text-xs">
                Click the button below to open the AlphaHuman bot in Telegram
              </p>
            </div>
          </div>
        </div>

        <div className="bg-stone-800/50 rounded-xl p-4 border border-stone-700">
          <div className="flex items-start space-x-3">
            <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
              <span className="text-white font-bold text-xs">2</span>
            </div>
            <div>
              <h3 className="font-semibold mb-1 text-sm">Keep this tab open</h3>
              <p className="opacity-70 text-xs">
                Keep this tab open to keep the connection alive. You can message the assistant with the button below.
              </p>
            </div>
          </div>
        </div>

        <div className="bg-stone-800/50 rounded-xl p-4 border border-stone-700">
          <div className="flex items-start space-x-3">
            <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center flex-shrink-0">
              <span className="text-white font-bold text-xs">3</span>
            </div>
            <div>
              <h3 className="font-semibold mb-1 text-sm">Start Messaging</h3>
              <p className="opacity-70 text-xs">
                Send a message to the bot to get started. Try asking about crypto prices, market trends, or anything about your chats!
              </p>
            </div>
          </div>
        </div>
      </div>

      <button
        onClick={handleOpenTelegram}
        className="w-full flex items-center justify-center space-x-3 bg-blue-500 hover:bg-blue-600 active:bg-blue-700 text-white font-semibold py-2.5 text-sm rounded-xl transition-all duration-300 hover:shadow-medium mb-3"
      >
        <span>I'm Ready! Let's Go! 🔥</span>
      </button>
    </div>
  );
};

export default GetStartedStep;
