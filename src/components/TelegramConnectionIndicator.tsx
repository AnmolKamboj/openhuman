import { useEffect, useState } from 'react';
import { useTelegram } from '../providers/TelegramProvider';

interface TelegramConnectionIndicatorProps {
  description?: string;
  className?: string;
}

const TelegramConnectionIndicator = ({
  description,
  className = '',
}: TelegramConnectionIndicatorProps) => {
  const { isAuthenticated, connectionStatus, checkConnection } = useTelegram();
  const [telegramIsOnline, setTelegramIsOnline] = useState(false);

  // Check Telegram connection status periodically
  useEffect(() => {
    if (!isAuthenticated) {
      setTelegramIsOnline(false);
      return;
    }

    const checkTelegramConnection = async () => {
      try {
        const isConnected = await checkConnection();
        setTelegramIsOnline(isConnected);
      } catch (error) {
        console.warn('Failed to check Telegram connection:', error);
        setTelegramIsOnline(false);
      }
    };

    // Check immediately if connected
    if (connectionStatus === 'connected') {
      checkTelegramConnection();
    } else {
      setTelegramIsOnline(false);
    }

    // Check every 20 seconds to keep user online and verify connection
    const interval = setInterval(checkTelegramConnection, 20000);

    return () => clearInterval(interval);
  }, [isAuthenticated, connectionStatus, checkConnection]);

  // Show indicator if authenticated or if there's a session
  if (!isAuthenticated) {
    return null;
  }

  return (
    <div className={`mb-6 ${className}`}>
      <div className="flex items-center justify-center space-x-2 mb-3">
        <div className={`w-2 h-2 ${telegramIsOnline ? 'bg-blue-500' : 'bg-gray-500'} rounded-full ${telegramIsOnline ? 'animate-pulse' : ''}`}></div>
        <div className="flex items-center space-x-1.5">
          <svg className="w-4 h-4 text-blue-500" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.568 8.16l-1.608 7.56c-.12.54-.432.672-.876.42l-2.424-1.788-1.17.732c-.132.084-.24.156-.492.156l.18-2.544 4.488-4.056c.192-.168-.042-.264-.3-.096l-5.544 3.492-2.388-.744c-.516-.162-.528-.516.108-.78l9.36-3.612c.432-.168.828.108.684.636z" />
          </svg>
          <span className={`text-sm ${telegramIsOnline ? 'text-blue-500' : 'text-gray-500'}`}>
            {telegramIsOnline ? 'Connected to Telegram' : 'Telegram is Offline'}
          </span>
        </div>
      </div>
      {description && (
        <p className="text-xs opacity-60 text-center leading-relaxed">
          {description}
        </p>
      )}
    </div>
  );
};

export default TelegramConnectionIndicator;
