import { useState } from 'react';

interface TelegramConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

type ConnectionStep = 'qr' | '2fa';

const TelegramConnectionModal = ({ isOpen, onClose, onComplete }: TelegramConnectionModalProps) => {
  const [currentStep, setCurrentStep] = useState<ConnectionStep>('qr');
  const [password, setPassword] = useState('');

  if (!isOpen) return null;

  const handleQRNext = () => {
    // In a real app, this would check if 2FA is required
    // For now, we'll simulate it by showing 2FA screen
    setCurrentStep('2fa');
  };

  const handle2FASubmit = () => {
    // In a real app, this would verify the password
    onComplete();
    onClose();
    // Reset state
    setCurrentStep('qr');
    setPassword('');
  };

  const handleBack = () => {
    if (currentStep === '2fa') {
      setCurrentStep('qr');
      setPassword('');
    } else {
      onClose();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
      <div className="relative w-full max-w-md mx-4">
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          {/* Close button */}
          <button
            onClick={onClose}
            className="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-full hover:bg-stone-800/50 transition-colors"
          >
            <svg className="w-5 h-5 opacity-70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>

          {currentStep === 'qr' ? (
            <>
              {/* QR Code Screen */}
              <div className="text-center">
                <h2 className="text-2xl font-bold mb-6">Log in to Telegram by QR Code</h2>

                {/* QR Code Container */}
                <div className="flex justify-center mb-8">
                  <div className="bg-white p-4 rounded-2xl shadow-large">
                    <div className="w-64 h-64 bg-white rounded-xl flex items-center justify-center relative overflow-hidden">
                      {/* QR code pattern - black squares on white background */}
                      <div className="absolute inset-0 p-2">
                        <div className="w-full h-full grid grid-cols-[repeat(25,minmax(0,1fr))] gap-0">
                          {Array.from({ length: 625 }).map((_, i) => {
                            const row = Math.floor(i / 25);
                            const col = i % 25;
                            // Create QR-like pattern with finder patterns in corners
                            const isFinder = 
                              (row < 7 && col < 7) || // Top-left
                              (row < 7 && col >= 18) || // Top-right
                              (row >= 18 && col < 7); // Bottom-left
                            // More realistic QR pattern - alternating pattern
                            const isBlack = isFinder || ((row + col) % 3 === 0 && Math.random() > 0.4) || (row * col % 7 < 3);
                            return (
                              <div
                                key={i}
                                className={`w-full h-full ${isBlack ? 'bg-black' : 'bg-white'}`}
                              />
                            );
                          })}
                        </div>
                      </div>
                      {/* Telegram logo in center */}
                      <div className="absolute inset-0 flex items-center justify-center z-10 pointer-events-none">
                        <div className="w-16 h-16 bg-blue-500 rounded-full flex items-center justify-center shadow-lg">
                          <svg className="w-10 h-10 text-white" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
                          </svg>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Instructions */}
                <div className="space-y-4 mb-6">
                  <div className="flex items-start space-x-3 text-left">
                    <div className="w-6 h-6 bg-purple-500 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
                      <span className="text-white font-bold text-xs">1</span>
                    </div>
                    <p className="opacity-90 text-sm">Open Telegram on your phone</p>
                  </div>

                  <div className="flex items-start space-x-3 text-left">
                    <div className="w-6 h-6 bg-purple-500 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
                      <span className="text-white font-bold text-xs">2</span>
                    </div>
                    <p className="opacity-90 text-sm">Go to Settings &gt; Devices &gt; Link Desktop Device</p>
                  </div>

                  <div className="flex items-start space-x-3 text-left">
                    <div className="w-6 h-6 bg-purple-500 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
                      <span className="text-white font-bold text-xs">3</span>
                    </div>
                    <p className="opacity-90 text-sm">Point your phone at this screen to confirm login</p>
                  </div>
                </div>

                {/* Action buttons */}
                <div className="flex space-x-3">
                  <button
                    onClick={handleBack}
                    className="flex-1 py-2.5 px-4 bg-stone-800/50 hover:bg-stone-700/50 border border-stone-700 rounded-xl text-sm font-medium transition-all duration-200"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleQRNext}
                    className="flex-1 py-2.5 px-4 bg-primary-500 hover:bg-primary-600 active:bg-primary-700 text-white rounded-xl text-sm font-medium transition-all duration-200"
                  >
                    I've Scanned
                  </button>
                </div>
              </div>
            </>
          ) : (
            <>
              {/* 2FA Screen */}
              <div className="text-center">
                <h2 className="text-2xl font-bold mb-2">Enter Your Password</h2>
                <p className="opacity-70 text-sm mb-6">
                  Your account is protected with two-step verification. Please enter your password to continue.
                </p>

                {/* Password input */}
                <div className="mb-6">
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="Enter your password"
                    className="w-full px-4 py-3 bg-black/50 border border-stone-700 rounded-xl text-white placeholder-opacity-50 focus:outline-none focus:border-primary-500 transition-colors"
                    autoFocus
                  />
                </div>

                {/* Action buttons */}
                <div className="flex space-x-3">
                  <button
                    onClick={handleBack}
                    className="flex-1 py-2.5 px-4 bg-stone-800/50 hover:bg-stone-700/50 border border-stone-700 rounded-xl text-sm font-medium transition-all duration-200"
                  >
                    Back
                  </button>
                  <button
                    onClick={handle2FASubmit}
                    disabled={!password.trim()}
                    className="flex-1 py-2.5 px-4 bg-primary-500 hover:bg-primary-600 active:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-xl text-sm font-medium transition-all duration-200"
                  >
                    Continue
                  </button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default TelegramConnectionModal;
