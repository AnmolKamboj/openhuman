import { type ReactElement, useCallback, useEffect, useRef, useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import { useExecuteTask } from '../../hooks/useIntelligenceApiFallback';
import { useIntelligenceSocket } from '../../hooks/useIntelligenceSocket';
import { getCurrentConnectedTools, initializeIntelligenceChatSession } from '../../lib/intelligence/chatTools';
import type { RootState } from '../../store';
import { addMessage, setTyping } from '../../store/intelligenceSlice';
import type { ChatModal as ChatModalType } from '../../types/intelligence';
import { createChatMessage } from '../../utils/intelligenceTransforms';

// Conversation flow states
type ConversationFlow =
  | 'discovery'
  | 'planning'
  | 'confirmation'
  | 'execution'
  | 'completion'
  | 'auto_close';

// Progress step interface
interface ProgressStep {
  id: string;
  label: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  duration?: number;
}

interface ChatModalProps {
  modal: ChatModalType;
  onClose: () => void;
  onComplete?: (itemId: string) => void;
}

// Legacy meeting preparation workflow removed - now handled by real backend AI

// Legacy demo script removed - now using real WebSocket integration

// Legacy AI response generator removed - now using real backend AI

export function ChatModal({ modal, onClose, onComplete }: ChatModalProps): ReactElement | null {
  const dispatch = useDispatch();

  // Redux state
  const activeSessions = useSelector((state: RootState) => state.intelligence.activeSessions);
  const activeExecutions = useSelector((state: RootState) => state.intelligence.activeExecutions);

  // Socket integration
  const { sendMessage, isConnected: socketConnected } = useIntelligenceSocket();

  // API hooks
  const { mutateAsync: executeTask } = useExecuteTask();

  // Local state
  const [inputValue, setInputValue] = useState('');
  const [conversationFlow, setConversationFlow] = useState<ConversationFlow>('discovery');
  const [progressSteps, setProgressSteps] = useState<ProgressStep[]>([]);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentExecutionId, setCurrentExecutionId] = useState<string | null>(null);

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const countdownInterval = useRef<ReturnType<typeof setInterval> | null>(null);

  // Get current session and execution data
  const currentThreadId = modal.item?.threadId;
  const currentSession = currentThreadId ? activeSessions[currentThreadId] : null;
  const currentExecution = currentExecutionId ? activeExecutions[currentExecutionId] : null;

  // Use messages from Redux state if available
  const messages = currentSession?.messages || modal.messages || [];
  const isTyping = currentSession?.isTyping || false;

  const executeTaskWithProgress = useCallback(async () => {
    if (!modal.item || !currentThreadId) return;

    try {
      setIsExecuting(true);
      setConversationFlow('execution');

      // Get available tools
      const connectedTools = getCurrentConnectedTools();

      if (connectedTools.length === 0) {
        // No tools available - send message to AI
        if (currentThreadId) {
          const errorMessage = createChatMessage(
            '❌ No tools are currently available. Please ensure skills are properly configured.',
            'ai'
          );
          dispatch(addMessage({ threadId: currentThreadId, message: errorMessage }));
        }
        setIsExecuting(false);
        return;
      }

      // Start task execution via backend
      const executionResult = await executeTask({
        itemId: modal.item.id,
        connectedTools,
      });

      setCurrentExecutionId(executionResult.executionId);

      // Send execution started message
      if (currentThreadId) {
        const startMessage = createChatMessage(
          '🚀 Starting task execution with available tools...',
          'ai'
        );
        dispatch(addMessage({ threadId: currentThreadId, message: startMessage }));
      }

      console.log('Intelligence: Task execution started', {
        executionId: executionResult.executionId,
        sessionId: executionResult.sessionId,
        toolCount: connectedTools.length,
      });
    } catch (error) {
      console.error('Error during task execution:', error);
      setIsExecuting(false);

      // Send error message
      if (currentThreadId) {
        const errorMessage = createChatMessage(
          '❌ Task execution failed. Please try again.',
          'ai'
        );
        dispatch(addMessage({ threadId: currentThreadId, message: errorMessage }));
      }
    }
  }, [modal.item, currentThreadId, executeTask, dispatch]);

  // Initialize real chat session
  const initializeChatSession = useCallback(() => {
    if (!modal.item?.threadId || !socketConnected) return;

    try {
      console.log('Intelligence: Initializing chat session', {
        itemId: modal.item.id,
        threadId: modal.item.threadId,
      });

      // Initialize chat session with available tools
      const sessionId = `chat_${modal.item.id}_${Date.now()}`;
      initializeIntelligenceChatSession(sessionId, modal.item.threadId);

      // Send welcome message if no existing messages
      if (messages.length === 0) {
        const welcomeMessage = createChatMessage(
          `I'll help you with "${modal.item.title}". What would you like me to do?`,
          'ai'
        );
        dispatch(addMessage({
          threadId: modal.item.threadId,
          message: welcomeMessage,
        }));
      }
    } catch (error) {
      console.error('Failed to initialize chat session:', error);
    }
  }, [modal.item, socketConnected, messages.length, dispatch]);

  // Initialize chat session when modal opens
  useEffect(() => {
    if (modal.isOpen && modal.item?.threadId && socketConnected) {
      // Initialize chat session after a brief delay
      setTimeout(() => {
        initializeChatSession();
      }, 500);
    }
  }, [modal.isOpen, modal.item?.threadId, socketConnected, initializeChatSession]);

  // Cleanup countdown interval
  useEffect(() => {
    return () => {
      // Clear countdown interval
      if (countdownInterval.current) {
        clearInterval(countdownInterval.current);
        countdownInterval.current = null;
      }
    };
  }, []);

  // Monitor execution progress from Redux state
  useEffect(() => {
    if (currentExecution && currentThreadId) {
      // Update local progress steps from Redux state
      if (currentExecution.progress.length > 0) {
        setProgressSteps(currentExecution.progress.map(step => ({
          id: step.id,
          label: step.label,
          status: step.status,
          duration: step.timestamp ? Date.now() - step.timestamp.getTime() : undefined,
        })));
      }

      // Handle execution completion
      if (currentExecution.status === 'completed') {
        setIsExecuting(false);
        setConversationFlow('completion');

        // Send completion message
        const completionMessage = createChatMessage(
          currentExecution.result
            ? `✅ Task completed successfully! ${currentExecution.result}`
            : '✅ Task completed successfully!',
          'ai'
        );
        dispatch(addMessage({ threadId: currentThreadId, message: completionMessage }));

        // Start auto-close countdown
        setTimeout(() => {
          setConversationFlow('auto_close');
          const closeMessage = createChatMessage('Closing this chat in 10 seconds...', 'ai');
          dispatch(addMessage({ threadId: currentThreadId, message: closeMessage }));
          setCountdown(10);
        }, 2000);
      } else if (currentExecution.status === 'failed') {
        setIsExecuting(false);
        const errorMessage = createChatMessage(
          `❌ Task execution failed: ${currentExecution.error || 'Unknown error'}`,
          'ai'
        );
        dispatch(addMessage({ threadId: currentThreadId, message: errorMessage }));
      }
    }
  }, [currentExecution, currentThreadId, dispatch]);

  // Focus input when modal opens
  useEffect(() => {
    if (modal.isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [modal.isOpen]);

  // Auto-close countdown
  useEffect(() => {
    if (countdown !== null && countdown > 0) {
      countdownInterval.current = setInterval(() => {
        setCountdown(prev => {
          if (prev === 1) {
            onClose();
            if (onComplete && modal.item) {
              onComplete(modal.item.id);
            }
            return 0;
          }
          return prev! - 1;
        });
      }, 1000);
    } else if (countdown === 0) {
      if (countdownInterval.current) {
        clearInterval(countdownInterval.current);
      }
    }

    return () => {
      if (countdownInterval.current) {
        clearInterval(countdownInterval.current);
      }
    };
  }, [countdown, onClose, onComplete, modal.item]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  // Handle manual execution trigger
  const handleExecutionTrigger = useCallback(() => {
    if (!isExecuting) {
      executeTaskWithProgress();
    }
  }, [isExecuting, executeTaskWithProgress]);

  const handleSend = useCallback(async () => {
    if (!inputValue.trim() || !currentThreadId || isExecuting || !socketConnected) return;

    const userMessage = inputValue.trim();
    setInputValue('');

    try {
      // Add user message to Redux state immediately
      const userChatMessage = createChatMessage(userMessage, 'user');
      dispatch(addMessage({ threadId: currentThreadId, message: userChatMessage }));

      // Show typing indicator
      dispatch(setTyping({ threadId: currentThreadId, isTyping: true }));

      // Send message via WebSocket
      await sendMessage({
        message: userMessage,
        threadId: currentThreadId,
        context: {
          conversationFlow,
          itemTitle: modal.item?.title,
          availableTools: getCurrentConnectedTools().length,
        },
      });

      console.log('Intelligence: Message sent via WebSocket', {
        threadId: currentThreadId,
        messageLength: userMessage.length,
        conversationFlow,
      });

      // Check if message indicates user wants to execute task
      const executionTriggers = [
        'yes',
        'proceed',
        'start',
        'execute',
        'go ahead',
        'do it',
        'let\'s go',
      ];

      if (executionTriggers.some(trigger => userMessage.toLowerCase().includes(trigger))) {
        // Trigger execution after AI responds
        setTimeout(() => {
          handleExecutionTrigger();
        }, 2000);
      }
    } catch (error) {
      console.error('Failed to send message:', error);

      // Remove typing indicator on error
      dispatch(setTyping({ threadId: currentThreadId, isTyping: false }));

      // Send error message to user
      const errorMessage = createChatMessage(
        '❌ Failed to send message. Please check your connection and try again.',
        'ai'
      );
      dispatch(addMessage({ threadId: currentThreadId, message: errorMessage }));
    }
  }, [
    inputValue,
    currentThreadId,
    isExecuting,
    socketConnected,
    conversationFlow,
    modal.item?.title,
    dispatch,
    sendMessage,
    handleExecutionTrigger,
  ]);

  const handleKeyPress = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        onClose();
      }
    },
    [onClose]
  );

  if (!modal.isOpen || !modal.item) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 animate-fade-in"
      onClick={handleBackdropClick}>
      <div
        className="bg-stone-900/95 backdrop-blur-xl rounded-2xl max-w-2xl w-full h-[600px] shadow-large border border-white/10 animate-slide-up flex flex-col"
        onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-white/10">
          <div className="flex-1 min-w-0">
            <h2 className="text-lg font-semibold text-white truncate">{modal.item.title}</h2>
            {modal.item.description && (
              <p className="text-sm text-stone-400 mt-1 truncate">{modal.item.description}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="ml-4 p-2 text-stone-400 hover:text-white rounded-lg hover:bg-white/10 transition-colors">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Chat Area */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {messages.map(message => (
            <div
              key={message.id}
              className={`flex ${message.sender === 'user' ? 'justify-end' : 'justify-start'}`}>
              <div
                className={`max-w-[80%] p-3 rounded-2xl animate-fade-in shadow-sm ring-1 ring-white/10 ${
                  message.sender === 'user'
                    ? 'bg-primary-500 text-white font-semibold ml-4'
                    : 'bg-stone-700/95 text-white font-semibold mr-4'
                }`}>
                <p className="text-sm leading-relaxed whitespace-pre-line">{message.content}</p>
                <p className="text-xs opacity-70 mt-1">
                  {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </p>
              </div>
            </div>
          ))}

          {/* Progress Steps Display */}
          {progressSteps.length > 0 && (
            <div className="flex justify-start">
              <div className="bg-stone-800/95 text-white mr-4 p-4 rounded-2xl shadow-sm ring-1 ring-white/10 max-w-[80%]">
                <h4 className="text-sm font-semibold mb-3 text-primary-400">🔄 Task Progress</h4>
                <div className="space-y-2">
                  {progressSteps.map((step, index) => (
                    <div key={step.id} className="flex items-center gap-3">
                      <div
                        className={`w-4 h-4 rounded-full flex items-center justify-center text-xs ${
                          step.status === 'completed'
                            ? 'bg-sage-500 text-white'
                            : step.status === 'in_progress'
                              ? 'bg-primary-500 text-white animate-pulse'
                              : step.status === 'failed'
                                ? 'bg-coral-500 text-white'
                                : 'bg-stone-600 text-stone-400'
                        }`}>
                        {step.status === 'completed'
                          ? '✓'
                          : step.status === 'in_progress'
                            ? '•'
                            : step.status === 'failed'
                              ? '✗'
                              : index + 1}
                      </div>
                      <span
                        className={`text-xs ${
                          step.status === 'completed'
                            ? 'text-sage-400'
                            : step.status === 'in_progress'
                              ? 'text-primary-400 font-medium'
                              : step.status === 'failed'
                                ? 'text-coral-400'
                                : 'text-stone-400'
                        }`}>
                        {step.label}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Countdown Display */}
          {countdown !== null && countdown > 0 && (
            <div className="flex justify-start">
              <div className="bg-amber-500/20 border border-amber-500/30 text-amber-200 mr-4 p-3 rounded-2xl shadow-sm max-w-[80%]">
                <div className="flex items-center gap-2">
                  <div className="w-6 h-6 rounded-full border-2 border-amber-400 flex items-center justify-center text-xs font-bold">
                    {countdown}
                  </div>
                  <span className="text-sm">Chat closing automatically...</span>
                </div>
              </div>
            </div>
          )}

          {isTyping && (
            <div className="flex justify-start">
              <div className="bg-stone-700/95 text-white font-semibold mr-4 p-3 rounded-2xl shadow-sm ring-1 ring-white/10">
                <div className="flex items-center space-x-1">
                  <div className="flex space-x-1">
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-pulse"
                      style={{ animationDelay: '0ms' }}
                    />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-pulse"
                      style={{ animationDelay: '200ms' }}
                    />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-pulse"
                      style={{ animationDelay: '400ms' }}
                    />
                  </div>
                </div>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Input Area */}
        <div className="p-6 border-t border-white/10 bg-stone-800/90">
          <div className="flex items-end gap-3">
            <div className="flex-1">
              <input
                ref={inputRef}
                type="text"
                value={inputValue}
                onChange={e => setInputValue(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder={
                  !socketConnected
                    ? 'Connecting...'
                    : isExecuting
                      ? 'Task is executing...'
                      : 'Type your message...'
                }
                className="w-full px-4 py-3 bg-stone-800/90 border border-stone-600/50 rounded-xl text-white font-medium placeholder-stone-300 focus:outline-none focus:border-primary-500/50 transition-colors resize-none shadow-sm"
                disabled={isTyping || !socketConnected || countdown !== null}
              />
            </div>
            <button
              onClick={handleSend}
              disabled={
                !inputValue.trim() ||
                isTyping ||
                !socketConnected ||
                countdown !== null
              }
              className="px-6 py-3 bg-primary-500 hover:bg-primary-600 disabled:bg-stone-700 disabled:cursor-not-allowed text-white rounded-lg transition-colors"
              title={
                !socketConnected
                  ? 'Connecting to server...'
                  : !inputValue.trim()
                    ? 'Enter a message'
                    : 'Send message'
              }>
              {isTyping ? (
                <div className="w-5 h-5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
              ) : (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                  />
                </svg>
              )}
            </button>
          </div>

          {/* Connection status indicator */}
          {!socketConnected && (
            <div className="mt-2 flex items-center gap-2 text-amber-400 text-xs">
              <div className="w-2 h-2 bg-amber-400 rounded-full animate-pulse" />
              <span>Establishing connection to AI system...</span>
            </div>
          )}
        </div>

        {/* Execution Status */}
        {isExecuting && (
          <div className="p-4 border-t border-white/10 bg-gradient-to-r from-primary-500/10 to-sage-500/10">
            <div className="flex items-center justify-center gap-2 text-primary-400">
              <div className="w-2 h-2 bg-primary-400 rounded-full animate-pulse" />
              <span className="text-sm font-medium">Task execution in progress...</span>
              <div
                className="w-2 h-2 bg-primary-400 rounded-full animate-pulse"
                style={{ animationDelay: '500ms' }}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
