import { createRoot } from 'react-dom/client';
import './index.css';

if (new URLSearchParams(location.search).has('dark')) {
  document.documentElement.classList.add('dark');
}

// Each row: a class whose colour SHOULD differ between themes, and one that
// should not, both with and without an opacity modifier (which is what
// compiles to color-mix(in oklab, …) under Tailwind v4).
const PROBES = [
  'text-content',
  'text-content/80',
  'text-content-muted',
  'text-content-muted/80',
  'bg-surface',
  'bg-surface/80',
  'text-primary-300',
  'text-primary-300/80',
  'text-primary-700/80',
  'border-content-faint/35',
];

createRoot(document.getElementById('root')!).render(
  <div className="bg-surface p-2">
    {PROBES.map(c => (
      <div key={c} data-probe={c} className={`border ${c}`}>
        {c}
      </div>
    ))}
  </div>
);
