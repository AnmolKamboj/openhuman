'use client';

import type { Feedback } from '../feedback/page';

interface FeedbackCardProps {
    feedback: Feedback;
    token: string | null;
    onUpdate: () => void;
    onClick: () => void;
}

export default function FeedbackCard({ feedback, token, onUpdate, onClick }: FeedbackCardProps) {
    const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

    const handleVote = async (voteType: 'upvote' | 'downvote') => {
        if (!token) return;

        try {
            const response = await fetch(`${API_BASE_URL}/api/feedback/${feedback._id}/vote`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`,
                },
                body: JSON.stringify({ voteType }),
            });

            const data = await response.json();
            if (data.success) {
                onUpdate();
            }
        } catch (error) {
            console.error('Failed to vote:', error);
        }
    };

    const getTypeLabel = (type: string) => {
        switch (type) {
            case 'feature':
                return 'FEATURE REQUEST';
            case 'bug':
                return 'BUG';
            case 'improvement':
                return 'IMPROVEMENT';
            default:
                return type.toUpperCase();
        }
    };

    return (
        <div
            className="cursor-pointer rounded-lg border border-zinc-800 bg-zinc-900/50 p-4 transition-colors hover:bg-zinc-900"
            onClick={onClick}
        >
            <div className="mb-2 flex items-start justify-between">
                <h3 className="font-semibold text-white">{feedback.title}</h3>
                <span className="ml-2 rounded bg-zinc-800 px-2 py-1 text-xs text-zinc-400">
                    {getTypeLabel(feedback.type)}
                </span>
            </div>
            {feedback.description && (
                <p className="mb-3 line-clamp-2 text-sm text-zinc-400">{feedback.description}</p>
            )}
            <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                    <button
                        onClick={(e) => {
                            e.stopPropagation();
                            handleVote('upvote');
                        }}
                        className={`rounded px-2 py-1 text-sm transition-colors ${feedback.userVote === 'upvote'
                            ? 'bg-blue-500 text-white'
                            : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'
                            }`}
                        disabled={!token}
                    >
                        ↑ {feedback.upvotes}
                    </button>
                    <button
                        onClick={(e) => {
                            e.stopPropagation();
                            handleVote('downvote');
                        }}
                        className={`rounded px-2 py-1 text-sm transition-colors ${feedback.userVote === 'downvote'
                            ? 'bg-red-500 text-white'
                            : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'
                            }`}
                        disabled={!token}
                    >
                        ↓ {feedback.downvotes}
                    </button>
                </div>
                <span className="text-xs text-zinc-500">
                    {feedback.comments.length} {feedback.comments.length === 1 ? 'comment' : 'comments'}
                </span>
            </div>
        </div>
    );
}
