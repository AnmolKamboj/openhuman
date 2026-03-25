import { useNavigate } from 'react-router-dom';

import RotatingTetrahedronCanvas from '../components/RotatingTetrahedronCanvas';

interface WelcomeProps {
  isWeb: boolean;
}

const Welcome = ({ isWeb }: WelcomeProps) => {
  const navigate = useNavigate();

  return (
    <div className="min-h-full w-full bg-[#090b12] px-4 py-6 md:px-10 md:py-10">
      <div className="mx-auto grid min-h-[calc(100vh-3.5rem)] w-full max-w-5xl grid-rows-[1fr_auto] border border-[#24293d] bg-[#0b0f18] text-white">
        <section className="relative grid place-items-center border-b border-[#1e2335] px-6 py-14">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,rgba(127,90,240,0.14),transparent_55%)]" />

          <div className="relative z-10 flex w-full max-w-2xl flex-col items-center gap-7 text-center">
            <div className="h-52 w-52 md:h-60 md:w-60">
              <RotatingTetrahedronCanvas />
            </div>

            <h1 className="text-balance text-4xl font-semibold tracking-tight text-white md:text-6xl">
              AlphaHuman
            </h1>

            <p className="max-w-xl text-sm text-[#8e96b8] md:text-base">
              A focused command center for AI-driven execution. Minimal surfaces, hard edges, and no
              visual noise.
            </p>

            <div className="flex flex-wrap items-center justify-center gap-3">
              <button
                className="border border-[#3d2f68] bg-[#201732] px-5 py-2 text-sm font-medium tracking-wide text-[#d4c8ff] transition-colors hover:bg-[#2a1d44]"
                type="button"
                onClick={() => navigate('/login')}>
                {isWeb ? 'Download AlphaHuman' : 'Continue'}
              </button>
              <button
                className="border border-[#2a3147] bg-[#121827] px-5 py-2 text-sm font-medium tracking-wide text-[#a7afce] transition-colors hover:bg-[#181f30]"
                type="button">
                View Docs
              </button>
            </div>
          </div>
        </section>

        <section className="grid grid-cols-1 md:grid-cols-2">
          <article className="border-b border-[#1e2335] p-6 md:border-b-0 md:border-r">
            <p className="mb-2 text-xs uppercase tracking-[0.2em] text-[#7362b0]">Documentation</p>
            <h2 className="text-2xl font-semibold tracking-tight">Built for depth</h2>
            <p className="mt-2 max-w-sm text-sm text-[#8e96b8]">
              Structured workflows, deliberate constraints, and precise outputs designed for power
              users.
            </p>
          </article>

          <article className="p-6">
            <p className="mb-2 text-xs uppercase tracking-[0.2em] text-[#7362b0]">Connect</p>
            <h2 className="text-2xl font-semibold tracking-tight">Join the mission</h2>
            <p className="mt-2 max-w-sm text-sm text-[#8e96b8]">
              Collaborate with the AlphaHuman community and shape what this assistant becomes next.
            </p>
          </article>
        </section>
      </div>
    </div>
  );
};

export default Welcome;
