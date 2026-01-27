import PrivacyFeatureCard from '../../../components/PrivacyFeatureCard';

interface FeaturesStepProps {
  onNext: () => void;
}

const FeaturesStep = ({ onNext }: FeaturesStepProps) => {
  const features = [
    {
      title: 'Keeps you on track',
      description: 'Organize your chats and tasks, finds you alpha and gets you deep insights. Get more done!',
    },
    {
      title: 'Has Infinite Memory & Learns',
      description: 'Your assistant can remember everything you tell it and learn from your interactions to help you get more done.',
    },
    {
      title: 'Trades the Trenches',
      description: 'Your assistant comes with it\'s own private wallet that can trade on any exchange for you.',
    },
  ];

  return (
    <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
      <div className="text-center mb-4">
        <h1 className="text-xl font-bold mb-2">Are You Ready For This?</h1>
        <p className="opacity-70 text-sm">
          Here are a few things that AlphaHuman can do that might surprise you.
        </p>
      </div>

      <div className="space-y-2 mb-4">
        {features.map((feature, index) => (
          <PrivacyFeatureCard
            key={index}
            title={feature.title}
            description={feature.description}
          />
        ))}
      </div>

      <button
        onClick={onNext}
        className="btn-primary w-full py-2.5 text-sm font-medium rounded-xl"
      >
        Yes I'm Ready. Bring It On 🚀
      </button>
    </div>
  );
};

export default FeaturesStep;
