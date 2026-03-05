import type { Message } from '../providers/interface';
import type { SoulConfig } from './types';

/**
 * Inject SOUL context into user message content.
 * Creates a seamless persona injection without modifying the original message structure.
 */
export function injectSoulIntoMessage(
  message: Message,
  soulConfig: SoulConfig,
  options: {
    mode: 'prepend' | 'context-block' | 'invisible';
    includeMetadata?: boolean;
  } = { mode: 'context-block' }
): Message {
  if (message.role !== 'user') {
    return message; // Only inject into user messages
  }

  const soulContext = buildSoulContext(soulConfig, options.includeMetadata);

  switch (options.mode) {
    case 'prepend':
      return {
        ...message,
        content: [
          { type: 'text', text: soulContext },
          ...message.content
        ]
      };

    case 'context-block':
      return {
        ...message,
        content: [
          { type: 'text', text: `[PERSONA_CONTEXT]\n${soulContext}\n[/PERSONA_CONTEXT]\n\nUser message:` },
          ...message.content
        ]
      };

    case 'invisible':
      // Add as hidden metadata that AI can access
      return {
        ...message,
        content: message.content.map((block, index) => {
          if (index === 0 && block.type === 'text') {
            return {
              ...block,
              text: `<!--SOUL_CONTEXT:${btoa(soulContext)}-->${block.text}`
            };
          }
          return block;
        })
      };

    default:
      return message;
  }
}

/**
 * Build compact SOUL context string optimized for token efficiency
 */
function buildSoulContext(soulConfig: SoulConfig, includeMetadata = false): string {
  const parts: string[] = [];

  // Core identity (always include)
  parts.push(`I am ${soulConfig.identity.name}: ${soulConfig.identity.description}`);

  // Key personality traits (top 3)
  if (soulConfig.personality.length > 0) {
    const topTraits = soulConfig.personality.slice(0, 3);
    parts.push(`Personality: ${topTraits.map(t => `${t.trait} (${t.description})`).join(', ')}`);
  }

  // Voice guidelines (condensed)
  if (soulConfig.voiceTone.length > 0) {
    const guidelines = soulConfig.voiceTone.slice(0, 2).map(v => v.guideline).join(', ');
    parts.push(`Voice: ${guidelines}`);
  }

  // Critical safety rules (priority > 8)
  const criticalRules = soulConfig.safetyRules.filter(r => r.priority > 8);
  if (criticalRules.length > 0) {
    parts.push(`Safety: ${criticalRules.map(r => r.rule).join('; ')}`);
  }

  if (includeMetadata) {
    parts.push(`Updated: ${new Date(soulConfig.loadedAt).toISOString()}`);
  }

  return parts.join('\n');
}

/**
 * Remove SOUL context from message (for display purposes)
 */
export function stripSoulFromMessage(message: Message): Message {
  if (message.role !== 'user') {
    return message;
  }

  return {
    ...message,
    content: message.content.map(block => {
      if (block.type === 'text') {
        // Remove context blocks
        let text = block.text.replace(/\[PERSONA_CONTEXT\][\s\S]*?\[\/PERSONA_CONTEXT\]\s*User message:\s*/g, '');
        // Remove invisible context
        text = text.replace(/<!--SOUL_CONTEXT:[^>]+-->/g, '');
        return { ...block, text };
      }
      return block;
    })
  };
}