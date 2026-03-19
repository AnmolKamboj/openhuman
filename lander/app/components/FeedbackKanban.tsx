'use client';

import { useState, useEffect } from 'react';
import FeedbackCard from './FeedbackCard';
import type { Feedback } from '../feedback/page';

interface FeedbackKanbanProps {
    feedbacks: Feedback[];
    token: string | null;
    onUpdate: () => void;
}

export default function FeedbackKanban({ feedbacks, token, onUpdate }: FeedbackKanbanProps) {
    const [selectedFeedback, setSelectedFeedback] = useState<Feedback | null>(null);

    const planned = feedbacks.filter((f) => f.status === 'planned');
    const inProgress = feedbacks.filter((f) => f.status === 'in_progress');
    const complete = feedbacks.filter((f) => f.status === 'complete');

    const columns = [
        {
            id: 'planned',
            title: 'Planned',
            items: planned,
            color: 'blue',
        },
        {
            id: 'in_progress',
            title: 'In Progress',
            items: inProgress,
            color: 'purple',
        },
        {
            id: 'complete',
            title: 'Complete',
            items: complete,
            color: 'green',
        },
    ];

    return (
        <>
            <div className="mb-6 flex items-center justify-between">
                <div className="flex items-center gap-4">
                    <h2 className="text-lg font-semibold">Boards</h2>
                    <div className="flex gap-2">
                        <button className="rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-2 text-sm text-zinc-400 transition-colors hover:bg-zinc-800">
                            Feature Requests <span className="ml-2 text-zinc-500">({feedbacks.filter((f) => f.type === 'feature').length})</span>
                        </button>
                        <button className="rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-2 text-sm text-zinc-400 transition-colors hover:bg-zinc-800">
                            Bugs <span className="ml-2 text-zinc-500">({feedbacks.filter((f) => f.type === 'bug').length})</span>
                        </button>
                    </div>
                </div>
                <button className="rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-2 text-sm text-zinc-400 transition-colors hover:bg-zinc-800">
                    Filters
                </button>
            </div>

            <div className="mb-8">
                <h2 className="mb-4 text-lg font-semibold">Roadmap</h2>
                <div className="grid gap-6 md:grid-cols-3">
                    {columns.map((column) => (
                        <div key={column.id} className="flex flex-col">
                            <div className="mb-4 flex items-center gap-2">
                                <div
                                    className={`h-2 w-2 rounded-full ${column.color === 'blue'
                                        ? 'bg-blue-500'
                                        : column.color === 'purple'
                                            ? 'bg-purple-500'
                                            : 'bg-green-500'
                                        }`}
                                />
                                <h3 className="font-semibold">{column.title}</h3>
                            </div>
                            <div className="space-y-4">
                                {column.items.length > 0 ? (
                                    column.items.map((feedback) => (
                                        <FeedbackCard
                                            key={feedback._id}
                                            feedback={feedback}
                                            token={token}
                                            onUpdate={onUpdate}
                                            onClick={() => setSelectedFeedback(feedback)}
                                        />
                                    ))
                                ) : (
                                    <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-8 text-center">
                                        <div className="mb-2 text-4xl">💡</div>
                                        <p className="text-sm text-zinc-400">
                                            Share your feedback and check back here for updates.
                                        </p>
                                    </div>
                                )}
                            </div>
                        </div>
                    ))}
                </div>
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
                />
            )}
        </>
    );
}

function FeedbackDetailModal({
    feedback,
    token,
    onClose,
    onUpdate,
}: {
    feedback: Feedback;
    token: string | null;
    onClose: () => void;
    onUpdate: () => void;
}) {
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

    const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

    const loadComments = async () => {
        try {
            const headers: HeadersInit = {};
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }

            const response = await fetch(`${API_BASE_URL}/api/feedback/${feedback._id}`, { headers });
            const data = await response.json();
            if (data.success) {
                setComments(data.data.comments || []);
            }
        } catch (error) {
            console.error('Failed to load comments:', error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadComments();
    }, [feedback._id, token]);

    const handleAddComment = async () => {
        if (!token || !newComment.trim()) return;

        try {
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
            }
        } catch (error) {
            console.error('Failed to add comment:', error);
        }
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
            <div className="w-full max-w-2xl rounded-lg border border-zinc-800 bg-zinc-900 max-h-[90vh] overflow-y-auto">
                <div className="sticky top-0 border-b border-zinc-800 bg-zinc-900 p-6">
                    <div className="flex items-start justify-between">
                        <div className="flex-1">
                            <h2 className="text-2xl font-semibold">{feedback.title}</h2>
                            {feedback.description && (
                                <p className="mt-2 text-zinc-400">{feedback.description}</p>
                            )}
                        </div>
                        <button
                            onClick={onClose}
                            className="ml-4 text-zinc-400 hover:text-white"
                        >
                            ✕
                        </button>
                    </div>
                </div>

                <div className="p-6">
                    <div className="mb-6">
                        <h3 className="mb-4 font-semibold">Comments</h3>
                        {loading ? (
                            <div className="text-zinc-400">Loading comments...</div>
                        ) : comments.length > 0 ? (
                            <div className="space-y-4">
                                {comments.map((comment) => (
                                    <div key={comment._id} className="rounded-lg border border-zinc-800 bg-zinc-950 p-4">
                                        <div className="mb-2 flex items-center gap-2">
                                            <span className="text-sm font-semibold">
                                                {comment.user.telegramFirstName || ''} {comment.user.telegramLastName || ''}
                                                {comment.user.telegramUsername && ` (@${comment.user.telegramUsername})`}
                                            </span>
                                            <span className="text-xs text-zinc-500">
                                                {new Date(comment.createdAt).toLocaleDateString()}
                                            </span>
                                        </div>
                                        <p className="text-sm text-zinc-300">{comment.content}</p>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="text-zinc-400">No comments yet.</div>
                        )}
                    </div>

                    {token && (
                        <div className="border-t border-zinc-800 pt-4">
                            <textarea
                                value={newComment}
                                onChange={(e) => setNewComment(e.target.value)}
                                className="w-full rounded-lg border border-zinc-800 bg-zinc-950 px-4 py-2 text-white focus:border-zinc-700 focus:outline-none"
                                rows={3}
                                placeholder="Add a comment..."
                            />
                            <button
                                onClick={handleAddComment}
                                className="mt-2 rounded-lg bg-white px-4 py-2 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-200"
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
