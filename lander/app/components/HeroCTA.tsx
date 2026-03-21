'use client';

import { useState } from 'react';
import Link from 'next/link';
import DownloadModal from './DownloadModal';

export default function HeroCTA() {
  const [downloadModalOpen, setDownloadModalOpen] = useState(false);

  return (
    <>
      <div className="mt-10 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-center sm:gap-4">
        <button
          type="button"
          onClick={() => setDownloadModalOpen(true)}
          className="rounded-lg cursor-pointer bg-white px-6 py-3 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-200"
        >
          Download 🔥
        </button>
        <Link
          href="https://x.com/tinyhumansai"
          target="_blank"
          rel="noopener noreferrer"
          className="rounded-lg border border-zinc-800 px-6 py-3 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
        >
          Join the Community
        </Link>
        <Link
          href="https://tinyhumans.gitbook.io/openhuman"
          target="_blank"
          rel="noopener noreferrer"
          className="rounded-lg border border-zinc-800 px-6 py-3 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
        >
          Read the Docs
        </Link>
      </div>
      <DownloadModal isOpen={downloadModalOpen} onClose={() => setDownloadModalOpen(false)} />
    </>
  );
}
