import HeroCTA from './components/HeroCTA';
import RotatingTetrahedronCanvas from './components/RotatingTetrahedronCanvas';

export default function Home() {
  return (
    <div className="min-h-screen bg-black text-white relative">
      <main className="mx-auto max-w-7xl px-6 sm:px-8 sm:pt-32 relative z-10">
        <div className="mx-auto max-w-3xl text-center">
          <div className="mx-auto mb-6 h-[180px] w-[180px] sm:h-[200px] sm:w-[200px]">
            <RotatingTetrahedronCanvas />
          </div>
          <h1 className="text-5xl font-bold tracking-tight sm:text-6xl lg:text-7xl">
            OpenHuman
          </h1>
          <p className="mt-6 text-lg leading-8 text-zinc-400 sm:text-xl">
            Your AI superhuman for your personal and business life. In private beta.
          </p>
          <HeroCTA />
        </div>

        <div className="mx-auto mt-24 max-w-5xl">
          <div className="grid gap-8 sm:grid-cols-2 lg:grid-cols-3 text-center">
            <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
              <h3 className="text-lg font-semibold text-white">Large Scale Data Analysis</h3>
              <p className="mt-2 text-sm text-zinc-400">
                Analyze all your emails, messages, meeting notes and more. Control the chaos.
              </p>
            </div>
            <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
              <h3 className="text-lg font-semibold text-white text-center">Extreme Intelligence</h3>
              <p className="mt-2 text-sm text-zinc-400">
                Get insights from your data that you never thought possible.
              </p>
            </div>
            <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
              <h3 className="text-lg font-semibold text-white text-center">Take Action</h3>
              <p className="mt-2 text-sm text-zinc-400">
                Let OpenHuman do the heavy lifting. Automatically create tasks,
                send messages, get reports, and more.
              </p>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
