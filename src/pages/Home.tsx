import { useState } from 'react';

const Home = () => {
  const [userName] = useState('Cyrus');

  // Get current date
  const getCurrentDate = () => {
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const now = new Date();
    return `${days[now.getDay()]}, ${months[now.getMonth()]} ${now.getDate()}`;
  };

  // Get greeting based on time
  const getGreeting = () => {
    const hour = new Date().getHours();
    if (hour < 12) return 'Good morning';
    if (hour < 18) return 'Good afternoon';
    return 'Good evening';
  };

  return (
    <div className="min-h-screen relative overflow-hidden">
      {/* Beach background image */}
      <div
        className="absolute inset-0 bg-cover bg-center bg-no-repeat"
        style={{
          backgroundImage: `linear-gradient(rgba(59, 130, 246, 0.1), rgba(16, 185, 129, 0.1)), url("data:image/svg+xml,%3Csvg width='1920' height='1080' viewBox='0 0 1920 1080' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cdefs%3E%3ClinearGradient id='sky' x1='0%25' y1='0%25' x2='0%25' y2='100%25'%3E%3Cstop offset='0%25' stop-color='%2387CEEB'/%3E%3Cstop offset='100%25' stop-color='%23E0F6FF'/%3E%3C/linearGradient%3E%3ClinearGradient id='ocean' x1='0%25' y1='0%25' x2='0%25' y2='100%25'%3E%3Cstop offset='0%25' stop-color='%2320B2AA'/%3E%3Cstop offset='100%25' stop-color='%234682B4'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='1920' height='600' fill='url(%23sky)'/%3E%3Cellipse cx='1920' cy='150' rx='80' ry='80' fill='%23FFD700' opacity='0.9'/%3E%3Cpath d='M0 400 Q480 350 960 380 T1920 400 V600 Q1440 580 960 590 T0 600 Z' fill='url(%23ocean)'/%3E%3Cpath d='M0 500 Q480 480 960 490 T1920 500 V1080 H0 Z' fill='%23F4A460'/%3E%3C/svg%3E")`
        }}
      ></div>

      {/* Content overlay */}
      <div className="relative z-10 min-h-screen flex flex-col">
        {/* Main content */}
        <div className="flex-1 flex items-center justify-center p-4">
          <div className="max-w-md w-full">
            {/* Weather card */}
            <div className="glass rounded-3xl p-8 shadow-large animate-fade-up text-center">
              {/* Palm tree icon */}
              <div className="w-16 h-16 mx-auto mb-6">
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

              {/* Date */}
              <p className="text-stone-600 text-sm mb-2 font-medium">
                {getCurrentDate()}
              </p>

              {/* Greeting */}
              <h1 className="text-2xl font-bold text-stone-900 mb-2">
                {getGreeting()}, {userName}
              </h1>

              {/* Weather info */}
              <div className="flex items-center justify-center space-x-2 mb-8">
                <span className="text-3xl font-light text-stone-800">25°C</span>
                <span className="text-stone-600">and smoke</span>
              </div>

              {/* Get Access button */}
              <button className="btn-primary w-full py-4 text-lg font-semibold rounded-xl flex items-center justify-center space-x-2 hover:shadow-large transition-all duration-300 hover:scale-[1.02] active:scale-[0.98]">
                <svg className="w-6 h-6" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M18 10c0 3.866-3.582 7-8 7a8.841 8.841 0 01-4.083-.98L2 17l1.338-3.123C2.493 12.767 2 11.434 2 10c0-3.866 3.582-7 8-7s8 3.134 8 7zM7 9H5v2h2V9zm8 0h-2v2h2V9zM9 9h2v2H9V9z" clipRule="evenodd" />
                </svg>
                <span>Get Access</span>
              </button>

              {/* Additional info */}
              <div className="mt-6 flex items-center justify-center space-x-4 text-xs text-stone-500">
                <span className="flex items-center space-x-1">
                  <div className="w-2 h-2 bg-sage-500 rounded-full"></div>
                  <span>Secure Connection</span>
                </span>
                <span className="flex items-center space-x-1">
                  <div className="w-2 h-2 bg-primary-500 rounded-full"></div>
                  <span>Community Ready</span>
                </span>
              </div>
            </div>

            {/* Bottom action */}
            <div className="text-center mt-6">
              <p className="text-white/80 text-sm">
                Welcome to your crypto community dashboard
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Home;