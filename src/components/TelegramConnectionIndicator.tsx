import { useEffect, useState } from 'react';
import { useTelegram } from '../providers/TelegramProvider';
import TelegramIcon from '../assets/icons/telegram.svg';

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
          <img src={TelegramIcon} alt="Telegram" className={`w-4 h-4 ${telegramIsOnline ? 'opacity-100' : 'opacity-50'}`} />
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
