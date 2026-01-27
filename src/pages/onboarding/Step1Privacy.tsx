import { useNavigate } from 'react-router-dom';

const Step1Privacy = () => {
  const navigate = useNavigate();

  const handleContinue = () => {
    navigate('/onboarding/step2');
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
            <div className="w-8 h-8 bg-stone-700 rounded-full flex items-center justify-center text-white text-sm font-semibold">2</div>
            <div className="w-12 h-1 bg-stone-700 mx-2"></div>
          </div>
          <div className="flex items-center">
            <div className="w-8 h-8 bg-stone-700 rounded-full flex items-center justify-center text-white text-sm font-semibold">3</div>
            <div className="w-12 h-1 bg-stone-700 mx-2"></div>
          </div>
          <div className="w-8 h-8 bg-stone-700 rounded-full flex items-center justify-center text-white text-sm font-semibold">4</div>
        </div>

        {/* Privacy card */}
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          <div className="text-center mb-8">
            <h1 className="text-2xl font-bold mb-2">
              Privacy
            </h1>
            <p className="opacity-70">
              Your security and privacy are our top priorities
            </p>
          </div>

          {/* Enterprise-Grade Security Section */}
          <div className="space-y-6 mb-8">
            {/* Enterprise-Grade Security */}
            <div className="bg-stone-800/50 rounded-xl p-6 border border-stone-700">
              <div className="flex items-start space-x-4">
                <div className="w-12 h-12 bg-sage-500 rounded-xl flex items-center justify-center flex-shrink-0">
                  <svg className="w-6 h-6 text-white" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 1L5 6v4c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V6l-5-5z" />
                  </svg>
                </div>
                <div>
                  <h3 className="font-semibold mb-2">Enterprise-Grade Security</h3>
                  <p className="opacity-70 text-sm leading-relaxed">
                    Bank-level encryption, multi-factor authentication, and 24/7 security monitoring to protect your assets and personal information.
                  </p>
                </div>
              </div>
            </div>

            {/* Privacy Section */}
            <div className="bg-stone-800/50 rounded-xl p-6 border border-stone-700">
              <div className="flex items-start space-x-4">
                <div className="w-12 h-12 bg-primary-500 rounded-xl flex items-center justify-center flex-shrink-0">
                  <svg className="w-6 h-6 text-white" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M3.707 2.293a1 1 0 00-1.414 1.414l14 14a1 1 0 001.414-1.414l-1.473-1.473A10.014 10.014 0 0019.542 10C18.268 5.943 14.478 3 10 3a9.958 9.958 0 00-4.512 1.074l-1.78-1.781zm4.261 4.26l1.514 1.515a2.003 2.003 0 012.45 2.45l1.514 1.514a4 4 0 00-5.478-5.478z" clipRule="evenodd" />
                    <path d="M12.454 16.697L9.75 13.992a4 4 0 01-3.742-3.741L2.335 6.578A9.98 9.98 0 00.458 10c1.274 4.057 5.065 7 9.542 7 .847 0 1.669-.105 2.454-.303z" />
                  </svg>
                </div>
                <div>
                  <h3 className="font-semibold mb-2">Privacy</h3>
                  <p className="opacity-70 text-sm leading-relaxed">
                    Zero-knowledge architecture ensures your trading patterns and holdings remain completely private from third parties.
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Continue button */}
          <button
            onClick={handleContinue}
            className="btn-primary w-full py-4 text-lg font-semibold rounded-xl"
          >
            Continue
          </button>
        </div>
      </div>
    </div>
  );
};

export default Step1Privacy;
