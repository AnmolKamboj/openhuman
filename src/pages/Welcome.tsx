import TypewriterGreeting from "../components/TypewriterGreeting";
import TelegramLoginButton from "../components/TelegramLoginButton";

const Welcome = () => {
  const greetings = [
    "Hello Satoshi! 👋",
    "Got Crypto, Anon? 👀",
    "Let's cook! 🔥",
    "Let's Ape Together! 👊",
    // "Welcome to the exclusive club of crypto degenerates! 🎪🚀",
    // "Let's get you richer than a Nigerian prince's email! 👑💸",
    // "Ready to HODL like your life depends on it? 🤝💀",
    // "Welcome, future crypto millionaire (results not guaranteed)! 🎰💎",
    // "Time to make Wall Street bros jealous AF! 📈🔥",
    // "Ready to go to the moon? Pack light! 🌙🚀"
  ];

  return (
    <div className="min-h-screen relative flex items-center justify-center">
      {/* Main content */}
      <div className="relative z-10 max-w-md w-full mx-4">
        {/* Welcome card */}
        <div className="glass rounded-3xl p-8 text-center animate-fade-up shadow-large">
          {/* Greeting */}
          <TypewriterGreeting greetings={greetings} />

          {/* <br /> */}

          <p className="opacity-70 mb-8 leading-relaxed">
            Welcome to AlphaHuman. Your Telegram assistant here to get you 10x
            more done in your crypto journey.
          </p>

          <p className="opacity-70 mb-8 leading-relaxed">
            Are you ready to cook?
          </p>

          {/* Login with Telegram button */}
          <TelegramLoginButton />
        </div>

        {/* Bottom text */}
        <p className="text-center opacity-60 text-sm mt-6">
          Made with ❤️ by a bunch of Web3 nerds
        </p>
      </div>
    </div>
  );
};

export default Welcome;
