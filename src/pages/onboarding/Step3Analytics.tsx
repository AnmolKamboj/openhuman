import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

const Step3Analytics = () => {
  const navigate = useNavigate();
  const [selectedOption, setSelectedOption] = useState('maximumPrivacy');

  const handleContinue = () => {
    navigate('/onboarding/step4');
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-primary-200 via-sage-100 to-amber-100 relative flex items-center justify-center">
      {/* Background pattern */}
      <div className="absolute inset-0 bg-noise opacity-30"></div>

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
          <div className="w-8 h-8 bg-primary-500 rounded-full flex items-center justify-center text-white text-sm font-semibold">3</div>
        </div>

        {/* Analytics card */}
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          <div className="text-center mb-8">
            <h1 className="text-2xl font-bold text-stone-900 mb-2">
              Analytics
            </h1>
            <p className="text-stone-600">
              Help us improve your experience while maintaining your privacy
            </p>
          </div>

          {/* Analytics options */}
          <div className="space-y-4 mb-8">
            {/* Securely Share Analytics */}
            <div
              className={`p-4 rounded-xl border-2 cursor-pointer transition-all ${
                selectedOption === 'shareAnalytics'
                  ? 'border-primary-500 bg-primary-50'
                  : 'border-stone-300 bg-white hover:border-stone-400'
              }`}
              onClick={() => setSelectedOption('shareAnalytics')}
            >
              <div className="flex items-start space-x-4">
                <div className="flex items-center justify-center mt-0.5">
                  <div className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                    selectedOption === 'shareAnalytics'
                      ? 'border-primary-500 bg-primary-500'
                      : 'border-stone-400 bg-white'
                  }`}>
                    {selectedOption === 'shareAnalytics' && (
                      <div className="w-2 h-2 bg-white rounded-full"></div>
                    )}
                  </div>
                </div>
                <div>
                  <h3 className="font-semibold text-stone-900 mb-1">Securely Share Analytics</h3>
                  <p className="text-stone-600 text-sm leading-relaxed">
                    Share anonymized usage data to help us improve features and performance. All data is encrypted and cannot be traced back to you.
                  </p>
                </div>
              </div>
            </div>

            {/* Maximum Privacy */}
            <div
              className={`p-4 rounded-xl border-2 cursor-pointer transition-all ${
                selectedOption === 'maximumPrivacy'
                  ? 'border-primary-500 bg-primary-50'
                  : 'border-stone-300 bg-white hover:border-stone-400'
              }`}
              onClick={() => setSelectedOption('maximumPrivacy')}
            >
              <div className="flex items-start space-x-4">
                <div className="flex items-center justify-center mt-0.5">
                  <div className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                    selectedOption === 'maximumPrivacy'
                      ? 'border-primary-500 bg-primary-500'
                      : 'border-stone-400 bg-white'
                  }`}>
                    {selectedOption === 'maximumPrivacy' && (
                      <div className="w-2 h-2 bg-white rounded-full"></div>
                    )}
                  </div>
                </div>
                <div>
                  <h3 className="font-semibold text-stone-900 mb-1">Maximum Privacy</h3>
                  <p className="text-stone-600 text-sm leading-relaxed">
                    Keep all your data completely private. We won't collect any usage analytics, ensuring total anonymity.
                  </p>
                  <div className="flex items-center space-x-1 mt-2">
                    <svg className="w-4 h-4 text-primary-600" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M10 1L5 6v4c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V6l-5-5z"/>
                    </svg>
                    <span className="text-primary-600 text-xs font-medium">Recommended for privacy</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Connect Email Account button */}
          <button
            onClick={handleContinue}
            className="btn-primary w-full py-4 text-lg font-semibold rounded-xl"
          >
            Connect Email Account
          </button>

          {/* Privacy note */}
          <div className="mt-6 p-4 bg-sage-50 rounded-xl border border-sage-200">
            <div className="flex items-start space-x-2">
              <svg className="w-5 h-5 text-sage-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd"/>
              </svg>
              <div>
                <p className="text-sage-800 font-medium text-sm">You can change this setting anytime</p>
                <p className="text-sage-600 text-xs mt-1">Your privacy preferences can be updated in your account settings</p>
              </div>
            </div>
          </div>
        </div>

        {/* Back button */}
        <button
          onClick={() => navigate('/onboarding/step2')}
          className="mt-6 w-full text-stone-500 hover:text-stone-700 text-sm font-medium transition-colors"
        >
          ← Back
        </button>
      </div>
    </div>
  );
};

export default Step3Analytics;