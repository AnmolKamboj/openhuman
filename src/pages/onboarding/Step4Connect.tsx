import { useNavigate } from 'react-router-dom';

const Step4Connect = () => {
  const navigate = useNavigate();

  const handleGoogleConnect = () => {
    // In a real app, this would handle Google OAuth
    navigate('/home');
  };

  const handleMicrosoftConnect = () => {
    // In a real app, this would handle Microsoft OAuth
    navigate('/home');
  };

  return (
    <div className="min-h-screen relative flex items-center justify-center">
      {/* Main content */}
      <div className="relative z-10 max-w-md w-full mx-4">
        {/* Connect Account card */}
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          {/* Palm tree icon */}
          <div className="text-center mb-8">
            <div className="w-16 h-16 mx-auto mb-4">
              <svg viewBox="0 0 64 64" fill="none" className="w-full h-full">
                {/* Palm tree trunk */}
                <path d="M28 32L30 54C30 56 32 58 34 58C36 58 38 56 38 54L40 32"
                      stroke="#8B5CF6" strokeWidth="3" strokeLinecap="round"/>

                {/* Palm fronds */}
                <path d="M32 32C32 32 20 18 14 16C12 15 10 17 12 19C18 25 30 30 32 32"
                      fill="#10B981"/>
                <path d="M32 32C32 32 46 18 52 16C54 15 56 17 54 19C48 25 36 30 34 32"
                      fill="#10B981"/>
                <path d="M32 32C32 32 38 10 40 4C41 2 43 2 43 4C41 10 35 28 32 32"
                      fill="#10B981"/>
                <path d="M32 32C32 32 26 10 24 4C23 2 21 2 21 4C23 10 29 28 32 32"
                      fill="#10B981"/>
                <path d="M32 32C32 32 50 24 56 22C58 21 58 23 56 25C50 27 34 32 32 32"
                      fill="#10B981"/>
                <path d="M32 32C32 32 14 24 8 22C6 21 6 23 8 25C14 27 30 32 32 32"
                      fill="#10B981"/>
              </svg>
            </div>

            <h1 className="text-2xl font-bold mb-2">
              Connect Account
            </h1>
            <p className="opacity-70">
              Connect your email to personalize your experience
            </p>
          </div>

          {/* Connection options */}
          <div className="space-y-4 mb-8">
            {/* Google */}
            <button
              onClick={handleGoogleConnect}
              className="w-full flex items-center justify-center space-x-3 p-4 bg-black/50 border border-stone-700 rounded-xl hover:border-stone-600 hover:shadow-medium transition-all duration-200 group"
            >
              <svg className="w-6 h-6" viewBox="0 0 24 24">
                <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
              </svg>
              <span className="font-medium">Use Google</span>
            </button>

            {/* Microsoft */}
            <button
              onClick={handleMicrosoftConnect}
              className="w-full flex items-center justify-center space-x-3 p-4 bg-black/50 border border-stone-700 rounded-xl hover:border-stone-600 hover:shadow-medium transition-all duration-200 group"
            >
              <svg className="w-6 h-6" viewBox="0 0 24 24">
                <path fill="#f25022" d="M1 1h10v10H1z"/>
                <path fill="#00a4ef" d="M13 1h10v10H13z"/>
                <path fill="#7fba00" d="M1 13h10v10H1z"/>
                <path fill="#ffb900" d="M13 13h10v10H13z"/>
              </svg>
              <span className="font-medium">Use Microsoft</span>
            </button>
          </div>

          {/* Skip option */}
          <button
            onClick={() => navigate('/home')}
            className="w-full py-3 opacity-60 hover:opacity-100 font-medium transition-opacity"
          >
            Skip for now
          </button>

          {/* Privacy note */}
          <div className="mt-6 p-4 bg-stone-800/50 rounded-xl border border-stone-700">
            <div className="flex items-start space-x-2">
              <svg className="w-5 h-5 text-primary-400 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 1L5 6v4c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V6l-5-5z"/>
              </svg>
              <div>
                <p className="font-medium text-sm">Your data stays private</p>
                <p className="opacity-70 text-xs mt-1">We only use your email for account notifications and security</p>
              </div>
            </div>
          </div>
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

export default Step4Connect;