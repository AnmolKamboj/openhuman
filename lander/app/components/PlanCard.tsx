
interface PlanCardProps {
  name: string;
  monthlyPrice: number;
  annualPrice: number;
  description: string;
  features: string[];
  cta: string;
  popular: boolean;
  billingCycle: 'monthly' | 'annual';
  onPrimaryAction?: () => void;
  onSecondaryAction?: () => void;
  secondaryCta?: string;
  disabled?: boolean;
  loading?: boolean;
}

export default function PlanCard({
  name,
  monthlyPrice,
  annualPrice,
  description,
  features,
  cta,
  popular,
  billingCycle,
  onPrimaryAction,
  onSecondaryAction,
  secondaryCta,
  disabled = false,
  loading = false,
}: PlanCardProps) {
  const price = billingCycle === 'monthly' ? monthlyPrice : annualPrice;
  const monthlyEquivalent = billingCycle === 'annual' && annualPrice > 0
    ? Math.round(annualPrice / 12)
    : monthlyPrice;
  const savings = billingCycle === 'annual' && monthlyPrice > 0
    ? (monthlyPrice * 12) - annualPrice
    : 0;

  return (
    <div
      className={`relative flex flex-col rounded-lg border p-8 ${popular
        ? 'border-white bg-zinc-900'
        : 'border-zinc-800 bg-zinc-900/50'
        }`}
    >
      {popular && (
        <div className="absolute -top-4 left-1/2 -translate-x-1/2">
          <span className="rounded-full bg-white px-3 py-1 text-xs font-semibold text-zinc-950">
            Most Popular
          </span>
        </div>
      )}
      <div className="text-center">
        <h3 className="text-2xl font-semibold text-white">{name}</h3>
        <div className="mt-4">
          {monthlyPrice === 0 ? (
            <div className="flex items-baseline justify-center gap-1">
              <span className="text-4xl font-bold text-white">$0</span>
              <span className="text-zinc-400">/forever</span>
            </div>
          ) : (
            <>
              <div className="flex items-baseline justify-center gap-1">
                <span className="text-4xl font-bold text-white">
                  ${price}
                </span>
                <span className="text-zinc-400">
                  /{billingCycle === 'monthly' ? 'month' : 'year'}
                </span>
              </div>
              {billingCycle === 'annual' && (
                <div className="mt-2">
                  <p className="text-sm text-zinc-400">
                    ${monthlyEquivalent}/month
                  </p>
                  <p className="mt-1 text-xs font-semibold text-green-400">
                    Save ${savings}/year
                  </p>
                </div>
              )}
            </>
          )}
        </div>
        <p className="mt-2 text-sm text-zinc-400">{description}</p>
      </div>
      <ul className="mt-8 flex-grow space-y-4">
        {features.map((feature, index) => (
          <li key={index} className="flex items-start gap-3">
            <svg
              className="mt-0.5 h-5 w-5 flex-shrink-0 text-white"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 13l4 4L19 7"
              />
            </svg>
            <span className="text-sm text-zinc-300">{feature}</span>
          </li>
        ))}
      </ul>
      <div className="mt-8 space-y-3">
        <button
          onClick={onPrimaryAction}
          disabled={disabled || !onPrimaryAction || loading}
          className={`w-full rounded-lg px-4 py-3 text-center text-sm font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
            popular
              ? 'bg-white text-zinc-950 hover:bg-zinc-200 disabled:hover:bg-white'
              : 'border border-zinc-800 text-white hover:border-zinc-700 disabled:hover:border-zinc-800'
          }`}
        >
          {loading ? 'Processing...' : cta}
        </button>
        {secondaryCta && onSecondaryAction && (
          <button
            onClick={onSecondaryAction}
            disabled={disabled || loading}
            className="w-full rounded-lg border border-zinc-800 px-4 py-3 text-center text-sm font-semibold text-zinc-200 transition-colors hover:border-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {secondaryCta}
          </button>
        )}
      </div>
    </div>
  );
}
