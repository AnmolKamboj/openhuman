import Navigation from '../components/Navigation';

export default function PrivacyPolicy() {
    return (
        <div className="min-h-screen bg-zinc-950 text-white">
            <Navigation />
            <main className="mx-auto max-w-4xl px-6 pt-24 sm:px-8 sm:pt-32 pb-16">
                <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
                    Privacy Policy
                </h1>
                <p className="mt-4 text-sm text-zinc-400">Last updated: {new Date().toLocaleDateString()}</p>

                <div className="mt-12 space-y-8 text-zinc-300">
                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">1. Information We Collect</h2>
                        <p className="leading-relaxed">
                            We collect information that you provide directly to us, including your name, email address,
                            and any other information you choose to provide when using our services.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">2. How We Use Your Information</h2>
                        <p className="leading-relaxed">
                            We use the information we collect to provide, maintain, and improve our services, process
                            transactions, send you technical notices and support messages, and respond to your comments
                            and questions.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">3. Information Sharing</h2>
                        <p className="leading-relaxed">
                            We do not sell, trade, or otherwise transfer your personal information to third parties
                            without your consent, except as described in this policy.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">4. Data Security</h2>
                        <p className="leading-relaxed">
                            We implement appropriate technical and organizational measures to protect your personal
                            information against unauthorized access, alteration, disclosure, or destruction.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">5. Your Rights</h2>
                        <p className="leading-relaxed">
                            You have the right to access, update, or delete your personal information at any time.
                            You may also opt out of certain communications from us.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">6. Contact Us</h2>
                        <p className="leading-relaxed">
                            If you have any questions about this Privacy Policy, please contact us at{' '}
                            <a href="mailto:privacy@openhuman.xyz" className="text-white underline hover:text-zinc-300">
                                privacy@openhuman.xyz
                            </a>
                        </p>
                    </section>
                </div>
            </main>
        </div>
    );
}
