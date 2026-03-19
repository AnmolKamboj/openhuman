import Link from 'next/link';

export default function Navigation() {
    return (
        <nav className="fixed top-0 left-0 right-0 z-50 border-b border-zinc-800 bg-zinc-950/80 backdrop-blur-sm">
            <div className="mx-auto max-w-7xl px-6 sm:px-8">
                <div className="flex h-16 items-center justify-between">
                    <Link href="/" className="text-xl font-semibold text-white">
                        OpenHuman
                    </Link>
                    <div className="flex items-center gap-4">
                        <Link
                            href="/"
                            className="text-sm text-zinc-400 transition-colors hover:text-white"
                        >
                            Home
                        </Link>
                        <Link
                            href="/pricing"
                            className="text-sm text-zinc-400 transition-colors hover:text-white"
                        >
                            Pricing
                        </Link>
                        <Link
                            href="/downloads"
                            className="rounded-lg bg-white px-4 py-2 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-200"
                        >
                            Download
                        </Link>
                    </div>
                </div>
            </div>
        </nav>
    );
}
