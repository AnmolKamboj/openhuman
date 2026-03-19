'use client';

interface TelegramLoginProps {
    onAuth: (user: {
        id: number;
        first_name?: string;
        last_name?: string;
        username?: string;
        auth_date: number;
        hash: string;
    }) => void;
    botName?: string;
}

export default function TelegramLogin({ onAuth }: TelegramLoginProps) {
    const handleButtonClick = () => {
        // Mock Telegram user data for testing
        const mockUser = {
            id: 123456789,
            first_name: 'John',
            last_name: 'Doe',
            username: 'johndoe',
            auth_date: Math.floor(Date.now() / 1000),
            hash: 'mock_hash_for_testing_' + Date.now(),
        };

        // Simulate a small delay like real authentication would have
        setTimeout(() => {
            onAuth(mockUser);
        }, 300);
    };

    return (
        <button
            onClick={handleButtonClick}
            className="flex items-center gap-1.5 sm:gap-2 rounded-lg bg-[#0088cc] px-3 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm font-semibold text-white transition-all active:bg-[#006699] sm:hover:bg-[#0077b3] shadow-lg sm:hover:shadow-xl sm:hover:scale-105 touch-manipulation"
            type="button"
        >
            <svg className="h-4 w-4 sm:h-5 sm:w-5 flex-shrink-0" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.894 8.221l-1.97 9.28c-.145.658-.537.818-1.084.508l-3-2.21-1.446 1.394c-.14.18-.357.295-.6.295-.002 0-.003 0-.005 0l.213-3.054 5.56-5.022c.24-.213-.054-.334-.373-.12l-6.869 4.326-2.96-.924c-.64-.203-.658-.64.135-.954l11.566-4.458c.538-.196 1.006.128.832.941z" />
            </svg>
            <span className="hidden sm:inline">Log in with Telegram</span>
            <span className="sm:hidden">Login</span>
        </button>
    );
}
