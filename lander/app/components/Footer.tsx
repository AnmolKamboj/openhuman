export default function Footer() {
    return (
        <footer className="border-t border-zinc-800 bg-zinc-950">
            <div className="mx-auto max-w-7xl px-6 py-6 sm:px-8">
                <div className="flex flex-wrap items-center justify-center gap-4 text-sm text-zinc-400">
                    <span>© {new Date().getFullYear()} AlphaHuman</span>
                    <span className="text-zinc-600">•</span>
                    <a
                        href="https://alphahuman.xyz/privacy"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="transition-colors hover:text-white"
                    >
                        Privacy Policy
                    </a>
                    <span className="text-zinc-600">•</span>
                    <a
                        href="https://alphahuman.xyz/terms"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="transition-colors hover:text-white"
                    >
                        Terms & Conditions
                    </a>
                </div>
            </div>
        </footer>
    );
}
