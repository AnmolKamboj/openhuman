'use client';

import { useState } from 'react';
import type { Feedback } from '../feedback/page';
import FeedbackDetailModal from './FeedbackDetailModal';

interface FeedbackListProps {
    feedbacks: Feedback[];
    token: string | null;
    onUpdate: () => void;
    useMockData?: boolean;
    mockComments?: Record<string, Array<{
        _id: string;
        content: string;
        user: {
            telegramId: number;
            telegramFirstName?: string;
            telegramLastName?: string;
            telegramUsername?: string;
        };
        createdAt: string;
    }>>;
    setFeedbacks?: (feedbacks: Feedback[]) => void;
}

export default function FeedbackList({ feedbacks, token, onUpdate, useMockData = false, mockComments, setFeedbacks }: FeedbackListProps) {
    const [selectedFeedback, setSelectedFeedback] = useState<Feedback | null>(null);

    if (feedbacks.length === 0) {
        return (
            <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-8 sm:p-12 text-center">
                <p className="text-sm sm:text-base text-zinc-400">No {feedbacks.length === 0 ? 'feedback' : 'results'} found.</p>
            </div>
        );
    }

    return (
        <>
            <div className="space-y-3 sm:space-y-4">
                {feedbacks.map((feedback) => (
                    <FeedbackListItem
                        key={feedback._id}
                        feedback={feedback}
                        token={token}
                        onUpdate={onUpdate}
                        onClick={() => setSelectedFeedback(feedback)}
                        useMockData={useMockData}
                        setFeedbacks={setFeedbacks}
                        allFeedbacks={feedbacks}
                    />
                ))}
            </div>

            {selectedFeedback && (
                <FeedbackDetailModal
                    feedback={selectedFeedback}
                    token={token}
                    onClose={() => setSelectedFeedback(null)}
                    onUpdate={() => {
                        onUpdate();
                        setSelectedFeedback(null);
                    }}
                    useMockData={useMockData}
                    mockComments={mockComments}
                    feedbacks={feedbacks}
                    setFeedbacks={setFeedbacks}
                />
            )}
        </>
    );
}

function FeedbackListItem({
    feedback,
    token,
    onUpdate,
    onClick,
    useMockData = false,
    setFeedbacks,
    allFeedbacks,
}: {
    feedback: Feedback;
    token: string | null;
    onUpdate: () => void;
    onClick: () => void;
    useMockData?: boolean;
    setFeedbacks?: (feedbacks: Feedback[]) => void;
    allFeedbacks: Feedback[];
}) {
    const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

    const handleVote = async (e: React.MouseEvent, voteType: 'upvote' | 'downvote') => {
        e.stopPropagation();
        if (!token) return;

        try {
            if (useMockData && setFeedbacks) {
                // Mock vote - update local state
                await new Promise(resolve => setTimeout(resolve, 200));

                const updatedFeedbacks = allFeedbacks.map(f => {
                    if (f._id === feedback._id) {
                        const updated = { ...f };

                        // Toggle vote logic
                        if (updated.userVote === voteType) {
                            // Remove vote
                            updated.userVote = null;
                            if (voteType === 'upvote') {
                                updated.upvotes = Math.max(0, updated.upvotes - 1);
                            } else {
                                updated.downvotes = Math.max(0, updated.downvotes - 1);
                            }
                        } else if (updated.userVote) {
                            // Change vote
                            const oldVote = updated.userVote;
                            if (oldVote === 'upvote') {
                                updated.upvotes = Math.max(0, updated.upvotes - 1);
                            } else {
                                updated.downvotes = Math.max(0, updated.downvotes - 1);
                            }
                            updated.userVote = voteType;
                            if (voteType === 'upvote') {
                                updated.upvotes += 1;
                            } else {
                                updated.downvotes += 1;
                            }
                        } else {
                            // New vote
                            updated.userVote = voteType;
                            if (voteType === 'upvote') {
                                updated.upvotes += 1;
                            } else {
                                updated.downvotes += 1;
                            }
                        }

                        return updated;
                    }
                    return f;
                });

                // Update global mock state
                const { setMockFeedbacks } = require('../feedback/page');
                setMockFeedbacks(updatedFeedbacks);
                setFeedbacks(updatedFeedbacks);
            } else {
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
            }
        } catch (error) {
            console.error('Failed to vote:', error);
        }
    };

    const getStatusColor = (status: string) => {
        switch (status) {
            case 'planned':
                return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
            case 'in_progress':
                return 'bg-purple-500/20 text-purple-400 border-purple-500/30';
            case 'complete':
                return 'bg-green-500/20 text-green-400 border-green-500/30';
            default:
                return 'bg-zinc-800 text-zinc-400 border-zinc-700';
        }
    };

    return (
        <div
            className="cursor-pointer rounded-lg border border-zinc-800 bg-zinc-900/50 p-4 sm:p-6 transition-colors active:bg-zinc-900 sm:hover:bg-zinc-900"
            onClick={onClick}
        >
            <div className="flex items-start justify-between gap-3 sm:gap-4">
                <div className="flex-1 min-w-0">
                    <div className="mb-2 flex flex-wrap items-center gap-2 sm:gap-3">
                        <h3 className="text-base sm:text-lg font-semibold text-white break-words">{feedback.title}</h3>
                        <span className={`rounded-full border px-2 py-0.5 sm:py-1 text-xs font-medium whitespace-nowrap ${getStatusColor(feedback.status)}`}>
                            {feedback.status === 'in_progress' ? 'In Progress' : feedback.status.charAt(0).toUpperCase() + feedback.status.slice(1)}
                        </span>
                    </div>
                    {feedback.description && (
                        <p className="mb-3 sm:mb-4 line-clamp-2 text-xs sm:text-sm text-zinc-400">{feedback.description}</p>
                    )}
                    <div className="flex items-center gap-3 sm:gap-4">
                        <button
                            onClick={(e) => handleVote(e, 'upvote')}
                            className={`flex items-center gap-1.5 sm:gap-2 rounded-lg px-2.5 sm:px-3 py-1.5 text-xs sm:text-sm transition-colors touch-manipulation ${feedback.userVote === 'upvote'
                                ? 'bg-blue-500/20 text-blue-400'
                                : 'bg-zinc-800 text-zinc-400 active:bg-zinc-700 sm:hover:bg-zinc-700'
                                }`}
                            disabled={!token}
                        >
                            <svg className="h-3.5 w-3.5 sm:h-4 sm:w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                            </svg>
                            {feedback.upvotes}
                        </button>
                        <span className="text-xs sm:text-sm text-zinc-500">
                            {feedback.comments.length} {feedback.comments.length === 1 ? 'comment' : 'comments'}
                        </span>
                    </div>
                </div>
            </div>
        </div>
    );
}
