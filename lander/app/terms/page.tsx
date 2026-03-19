import Navigation from '../components/Navigation';

export default function TermsAndConditions() {
    return (
        <div className="min-h-screen bg-zinc-950 text-white">
            <Navigation />
            <main className="mx-auto max-w-4xl px-6 pt-24 sm:px-8 sm:pt-32 pb-16">
                <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
                    Terms & Conditions
                </h1>
                <p className="mt-4 text-sm text-zinc-400">Last updated: {new Date().toLocaleDateString()}</p>

                <div className="mt-12 space-y-8 text-zinc-300">
                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">1. Acceptance of Terms</h2>
                        <p className="leading-relaxed">
                            By accessing and using AlphaHuman, you accept and agree to be bound by the terms and
                            provision of this agreement.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">2. Use License</h2>
                        <p className="leading-relaxed">
                            Permission is granted to temporarily use AlphaHuman for personal, non-commercial
                            transitory viewing only. This is the grant of a license, not a transfer of title.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">3. Service Availability</h2>
                        <p className="leading-relaxed">
                            We strive to ensure our services are available 24/7, but we do not guarantee
                            uninterrupted access. We reserve the right to modify or discontinue services at any time.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">4. User Accounts</h2>
                        <p className="leading-relaxed">
                            You are responsible for maintaining the confidentiality of your account credentials and
                            for all activities that occur under your account.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">5. Payment Terms</h2>
                        <p className="leading-relaxed">
                            Subscription fees are billed in advance on a monthly or annual basis. All fees are
                            non-refundable except as required by law. You may cancel your subscription at any time.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">6. Limitation of Liability</h2>
                        <p className="leading-relaxed">
                            In no event shall AlphaHuman be liable for any indirect, incidental, special,
                            consequential, or punitive damages resulting from your use of the service.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">7. Changes to Terms</h2>
                        <p className="leading-relaxed">
                            We reserve the right to modify these terms at any time. Your continued use of the
                            service after changes constitutes acceptance of the new terms.
                        </p>
                    </section>

                    <section>
                        <h2 className="text-2xl font-semibold text-white mb-4">8. Contact Information</h2>
                        <p className="leading-relaxed">
                            If you have any questions about these Terms & Conditions, please contact us at{' '}
                            <a href="mailto:legal@alphahuman.xyz" className="text-white underline hover:text-zinc-300">
                                legal@alphahuman.xyz
                            </a>
                        </p>
                    </section>
                </div>
            </main>
        </div>
    );
}
