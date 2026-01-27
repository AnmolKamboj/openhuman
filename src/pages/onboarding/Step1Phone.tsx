import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { countries } from '../../data/countries';

const Step1Phone = () => {
  const navigate = useNavigate();
  const [selectedCountry, setSelectedCountry] = useState(countries[0]); // Default to US
  const [phoneNumber, setPhoneNumber] = useState('');
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  const formatPhoneNumber = (value: string) => {
    // Remove all non-digits
    const digits = value.replace(/\D/g, '');

    // Format as (XXX) XXX-XXXX for US numbers
    if (selectedCountry.code === 'US' && digits.length <= 10) {
      if (digits.length >= 6) {
        return `(${digits.slice(0, 3)}) ${digits.slice(3, 6)}-${digits.slice(6)}`;
      } else if (digits.length >= 3) {
        return `(${digits.slice(0, 3)}) ${digits.slice(3)}`;
      } else {
        return digits;
      }
    }

    // For other countries, return digits with spaces for readability
    return digits.replace(/(\d{3})/g, '$1 ').trim();
  };

  const handlePhoneChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const formatted = formatPhoneNumber(e.target.value);
    setPhoneNumber(formatted);
  };

  const handleCountrySelect = (country: typeof countries[0]) => {
    setSelectedCountry(country);
    setIsDropdownOpen(false);
    setPhoneNumber(''); // Clear phone number when country changes
  };

  const handleContinue = () => {
    if (phoneNumber.trim()) {
      navigate('/onboarding/step2');
    }
  };

  const handleTelegramContinue = () => {
    // Skip phone verification with Telegram
    navigate('/onboarding/step2');
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
            <div className="w-8 h-8 bg-stone-300 rounded-full flex items-center justify-center text-stone-500 text-sm font-semibold">2</div>
            <div className="w-12 h-1 bg-stone-300 mx-2"></div>
          </div>
          <div className="w-8 h-8 bg-stone-300 rounded-full flex items-center justify-center text-stone-500 text-sm font-semibold">3</div>
        </div>

        {/* Phone input card */}
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          <div className="text-center mb-8">
            <h1 className="text-2xl font-bold text-stone-900 mb-2">
              Verify Your Phone
            </h1>
            <p className="text-stone-600">
              We'll send you a secure verification code to protect your account
            </p>
          </div>

          {/* Country selector and phone input */}
          <div className="space-y-4 mb-6">
            {/* Country dropdown */}
            <div className="relative">
              <button
                onClick={() => setIsDropdownOpen(!isDropdownOpen)}
                className="w-full flex items-center justify-between p-4 bg-white border border-stone-300 rounded-xl hover:border-primary-500 transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
              >
                <div className="flex items-center space-x-3">
                  <span className="text-2xl">{selectedCountry.flag}</span>
                  <span className="font-medium text-stone-900">{selectedCountry.name}</span>
                  <span className="text-stone-500">{selectedCountry.dialCode}</span>
                </div>
                <svg className={`w-5 h-5 text-stone-500 transition-transform ${isDropdownOpen ? 'rotate-180' : ''}`} fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clipRule="evenodd" />
                </svg>
              </button>

              {/* Dropdown menu */}
              {isDropdownOpen && (
                <div className="absolute z-50 w-full mt-2 bg-white border border-stone-300 rounded-xl shadow-large max-h-60 overflow-y-auto">
                  {countries.map((country) => (
                    <button
                      key={country.code}
                      onClick={() => handleCountrySelect(country)}
                      className="w-full flex items-center space-x-3 p-3 hover:bg-stone-50 transition-colors text-left"
                    >
                      <span className="text-xl">{country.flag}</span>
                      <span className="font-medium text-stone-900 flex-1">{country.name}</span>
                      <span className="text-stone-500 text-sm">{country.dialCode}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Phone number input */}
            <div className="relative">
              <input
                type="tel"
                value={phoneNumber}
                onChange={handlePhoneChange}
                placeholder={selectedCountry.code === 'US' ? '(000) 000-0000' : 'Phone number'}
                className="input-primary rounded-xl pl-20"
              />
              <div className="absolute left-4 top-1/2 transform -translate-y-1/2 flex items-center space-x-2 text-stone-500">
                <span>{selectedCountry.flag}</span>
                <span className="text-sm">{selectedCountry.dialCode}</span>
              </div>
            </div>
          </div>

          {/* Continue button */}
          <button
            onClick={handleContinue}
            disabled={!phoneNumber.trim()}
            className="btn-primary w-full py-4 text-lg font-semibold rounded-xl mb-4 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Continue with phone
          </button>

          {/* Telegram alternative */}
          <button
            onClick={handleTelegramContinue}
            className="w-full flex items-center justify-center space-x-2 py-4 text-primary-600 font-medium rounded-xl border border-primary-200 hover:bg-primary-50 transition-colors"
          >
            <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
              <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
            </svg>
            <span>Continue with Telegram</span>
          </button>

          {/* Security note */}
          <div className="mt-6 p-4 bg-primary-50 rounded-xl border border-primary-200">
            <div className="flex items-start space-x-2">
              <svg className="w-5 h-5 text-primary-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 1L5 6v4c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V6l-5-5z"/>
              </svg>
              <div>
                <p className="text-primary-800 font-medium text-sm">Your Privacy Matters</p>
                <p className="text-primary-600 text-xs mt-1">Your phone number is encrypted and never shared with third parties</p>
              </div>
            </div>
          </div>
        </div>

        {/* Back button */}
        <button
          onClick={() => navigate('/login')}
          className="mt-6 w-full text-stone-500 hover:text-stone-700 text-sm font-medium transition-colors"
        >
          ← Back to login
        </button>
      </div>
    </div>
  );
};

export default Step1Phone;