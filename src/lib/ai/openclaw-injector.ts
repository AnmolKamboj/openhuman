/**
 * OpenClaw Context Injector
 *
 * Replaces the old injectAll() system. Instead of parsing markdown into
 * structured objects and building compact summaries, this injects the raw
 * markdown from all 7 ZeroClaw workspace files — matching the Rust backend's
 * `build_system_prompt()` approach exactly.
 */
import { buildOpenClawContext } from './openclaw-loader';

/**
 * Prepend ZeroClaw-style project context to a user message.
 *
 * @param userMessage - The raw user message string
 * @returns The message with OpenClaw context prepended
 */
export function injectOpenClawContext(userMessage: string): string {
  const context = buildOpenClawContext();

  if (!context) return userMessage;

  return `${context}\n\nUser message: ${userMessage}`;
}
