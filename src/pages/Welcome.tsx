import { useNavigate } from 'react-router-dom';

import RotatingTetrahedronCanvas from '../components/RotatingTetrahedronCanvas';

interface WelcomeProps {
  isWeb: boolean;
}

const Welcome = ({ isWeb }: WelcomeProps) => {
  const navigate = useNavigate();

  return (
    <div className="h-full w-full bg-[#090b12]">
      <div className="mx-auto grid h-full w-full max-w-5xl border border-[#24293d] bg-[#0b0f18] text-white">
        <section className="relative grid place-items-center px-6">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,rgba(127,90,240,0.14),transparent_55%)]" />

          <div className="relative z-10 flex w-full max-w-2xl flex-col items-center gap-7 text-center">
            <div className="h-52 w-52 md:h-60 md:w-60">
              <RotatingTetrahedronCanvas />
            </div>

            <h1 className="text-balance text-4xl font-semibold tracking-tight text-white md:text-6xl">
              OpenHuman
            </h1>

            <p className="max-w-xl text-sm text-[#8e96b8] md:text-base">
              Your AI superhuman for personal and business life.
            </p>

            <div className="flex flex-wrap items-center justify-center gap-3">
              <button
                className="border border-[#3d2f68] bg-[#201732] px-5 py-2 text-sm font-medium tracking-wide text-[#d4c8ff] transition-colors hover:bg-[#2a1d44]"
                type="button"
                onClick={() => navigate('/login')}>
                {isWeb ? 'Download OpenHuman' : 'Continue'}
              </button>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
};

export default Welcome;
