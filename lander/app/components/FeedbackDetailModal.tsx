'use client';

import { useState, useEffect } from 'react';
import type { Feedback } from '../feedback/page';

interface FeedbackDetailModalProps {
    feedback: Feedback;
    token: string | null;
    onClose: () => void;
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
    feedbacks?: Feedback[];
    setFeedbacks?: (feedbacks: Feedback[]) => void;
}

export default function FeedbackDetailModal({
    feedback,
    token,
    onClose,
    onUpdate,
    useMockData = false,
    mockComments,
    feedbacks = [],
    setFeedbacks,
}: FeedbackDetailModalProps) {
    const [comments, setComments] = useState<Array<{
        _id: string;
        content: string;
        user: {
            telegramId: number;
            telegramFirstName?: string;
            telegramLastName?: string;
            telegramUsername?: string;
        };
        createdAt: string;
    }>>([]);
    const [newComment, setNewComment] = useState('');
    const [loading, setLoading] = useState(true);
    const [currentFeedback, setCurrentFeedback] = useState(feedback);

    const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

    const loadComments = async () => {
        try {
            if (useMockData && mockComments) {
                // Mock comments
                await new Promise(resolve => setTimeout(resolve, 200));
                const feedbackComments = mockComments[feedback._id] || [];
                setComments(feedbackComments);
            } else {
                const headers: HeadersInit = {};
                if (token) {
                    headers['Authorization'] = `Bearer ${token}`;
                }

                const response = await fetch(`${API_BASE_URL}/api/feedback/${feedback._id}`, { headers });
                const data = await response.json();
                if (data.success) {
                    setComments(data.data.comments || []);
                    setCurrentFeedback(data.data);
                }
            }
        } catch (error) {
            console.error('Failed to load comments:', error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        setCurrentFeedback(feedback);
        loadComments();
    }, [feedback._id, token]);

    const handleAddComment = async () => {
        if (!token || !newComment.trim()) return;

        try {
            if (useMockData && mockComments && setFeedbacks) {
                // Mock add comment
                await new Promise(resolve => setTimeout(resolve, 200));
                const newCommentObj = {
                    _id: String(Date.now()),
                    content: newComment,
                    user: {
                        telegramId: 123456789,
                        telegramFirstName: 'John',
                        telegramLastName: 'Doe',
                        telegramUsername: 'johndoe',
                    },
                    createdAt: new Date().toISOString(),
                };

                if (!mockComments[feedback._id]) {
                    mockComments[feedback._id] = [];
                }
                mockComments[feedback._id].push(newCommentObj);

                // Update feedback comments count
                const updatedFeedbacks = feedbacks.map(f => {
                    if (f._id === feedback._id) {
                        return { ...f, comments: [...f.comments, newCommentObj._id] };
                    }
                    return f;
                });

                // Update global mock state
                const { setMockFeedbacks } = require('../feedback/page');
                setMockFeedbacks(updatedFeedbacks);
                setFeedbacks(updatedFeedbacks);

                setNewComment('');
                await loadComments();
                onUpdate();
            } else {
                const response = await fetch(`${API_BASE_URL}/api/feedback/${feedback._id}/comments`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Authorization': `Bearer ${token}`,
                    },
                    body: JSON.stringify({ content: newComment }),
                });

                const data = await response.json();
                if (data.success) {
                    setNewComment('');
                    await loadComments();
                    onUpdate();
                }
            }
        } catch (error) {
            console.error('Failed to add comment:', error);
        }
    };

    const handleVote = async (voteType: 'upvote' | 'downvote') => {
        if (!token) return;

        try {
            if (useMockData && setFeedbacks) {
                // Mock vote
                await new Promise(resolve => setTimeout(resolve, 200));

                const updatedFeedbacks = feedbacks.map(f => {
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

                        setCurrentFeedback(updated);
                        return updated;
                    }
                    return f;
                });

                // Update global mock state
                const { setMockFeedbacks } = require('../feedback/page');
                setMockFeedbacks(updatedFeedbacks);
                setFeedbacks(updatedFeedbacks);
                onUpdate();
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
                    setCurrentFeedback({
                        ...currentFeedback,
                        upvotes: data.data.upvotes,
                        downvotes: data.data.downvotes,
                        userVote: data.data.userVote,
                    });
                    onUpdate();
                }
            }
        } catch (error) {
            console.error('Failed to vote:', error);
        }
    };

    return (
        <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/50 backdrop-blur-sm p-0 sm:p-4">
            <div className="w-full sm:max-w-3xl rounded-t-2xl sm:rounded-lg border-t sm:border border-zinc-800 bg-zinc-900 max-h-[95vh] sm:max-h-[90vh] overflow-y-auto">
                <div className="sticky top-0 border-b border-zinc-800 bg-zinc-900 p-4 sm:p-6">
                    <div className="flex items-start justify-between gap-3">
                        <div className="flex-1 min-w-0">
                            <h2 className="text-lg sm:text-2xl font-semibold break-words">{currentFeedback.title}</h2>
                            {currentFeedback.description && (
                                <p className="mt-2 text-sm sm:text-base text-zinc-400 break-words">{currentFeedback.description}</p>
                            )}
                            <div className="mt-4 flex flex-wrap items-center gap-2 sm:gap-4">
                                <button
                                    onClick={() => handleVote('upvote')}
                                    className={`flex items-center gap-1.5 sm:gap-2 rounded-lg px-2.5 sm:px-3 py-1.5 text-xs sm:text-sm transition-colors touch-manipulation ${currentFeedback.userVote === 'upvote'
                                        ? 'bg-blue-500/20 text-blue-400'
                                        : 'bg-zinc-800 text-zinc-400 active:bg-zinc-700 sm:hover:bg-zinc-700'
                                        }`}
                                    disabled={!token}
                                >
                                    <svg className="h-3.5 w-3.5 sm:h-4 sm:w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                                    </svg>
                                    {currentFeedback.upvotes}
                                </button>
                                <button
                                    onClick={() => handleVote('downvote')}
                                    className={`flex items-center gap-1.5 sm:gap-2 rounded-lg px-2.5 sm:px-3 py-1.5 text-xs sm:text-sm transition-colors touch-manipulation ${currentFeedback.userVote === 'downvote'
                                        ? 'bg-red-500/20 text-red-400'
                                        : 'bg-zinc-800 text-zinc-400 active:bg-zinc-700 sm:hover:bg-zinc-700'
                                        }`}
                                    disabled={!token}
                                >
                                    <svg className="h-3.5 w-3.5 sm:h-4 sm:w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                                    </svg>
                                    {currentFeedback.downvotes}
                                </button>
                                <span className="text-xs sm:text-sm text-zinc-500">
                                    {comments.length} {comments.length === 1 ? 'comment' : 'comments'}
                                </span>
                            </div>
                        </div>
                        <button
                            onClick={onClose}
                            className="flex-shrink-0 p-2 text-zinc-400 active:text-white sm:hover:text-white touch-manipulation"
                        >
                            <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div className="p-4 sm:p-6">
                    <div className="mb-4 sm:mb-6">
                        <h3 className="mb-3 sm:mb-4 text-sm sm:text-base font-semibold">Comments</h3>
                        {loading ? (
                            <div className="text-sm text-zinc-400">Loading comments...</div>
                        ) : comments.length > 0 ? (
                            <div className="space-y-3 sm:space-y-4">
                                {comments.map((comment) => (
                                    <div key={comment._id} className="rounded-lg border border-zinc-800 bg-zinc-950 p-3 sm:p-4">
                                        <div className="mb-2 flex flex-wrap items-center gap-2">
                                            <span className="text-xs sm:text-sm font-semibold break-words">
                                                {comment.user.telegramFirstName || ''} {comment.user.telegramLastName || ''}
                                                {comment.user.telegramUsername && ` (@${comment.user.telegramUsername})`}
                                            </span>
                                            <span className="text-xs text-zinc-500">
                                                {new Date(comment.createdAt).toLocaleDateString()}
                                            </span>
                                        </div>
                                        <p className="text-xs sm:text-sm text-zinc-300 break-words">{comment.content}</p>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="text-sm text-zinc-400">No comments yet.</div>
                        )}
                    </div>

                    {token && (
                        <div className="border-t border-zinc-800 pt-4">
                            <textarea
                                value={newComment}
                                onChange={(e) => setNewComment(e.target.value)}
                                className="w-full rounded-lg border border-zinc-800 bg-zinc-950 px-4 py-2.5 text-sm sm:text-base text-white focus:border-zinc-700 focus:outline-none"
                                rows={3}
                                placeholder="Add a comment..."
                            />
                            <button
                                onClick={handleAddComment}
                                className="mt-2 w-full sm:w-auto rounded-lg bg-white px-4 py-2.5 text-sm font-semibold text-zinc-950 transition-colors active:bg-zinc-200 sm:hover:bg-zinc-200 touch-manipulation"
                            >
                                Post Comment
                            </button>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
