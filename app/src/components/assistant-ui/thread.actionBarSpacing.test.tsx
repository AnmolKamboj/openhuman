import {
  AssistantRuntimeProvider,
  type ThreadMessageLike,
  useExternalStoreRuntime,
} from '@assistant-ui/react';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Thread } from './thread';

/**
 * The assistant action bar reserves its own height (`min-h-*`) so that a bar
 * revealed on hover does not shift the transcript, and immediately cancels that
 * reservation with a matching negative bottom margin (`-mb-*`) so it costs
 * nothing in flow — inter-message spacing stays whatever the message group's
 * `gap-y-*` says.
 *
 * The pair is only self-cancelling while both halves sit on the same element.
 * They were separated once: the `-mb` drifted onto the message root, where it
 * merely cancelled that element's own paint-box `pb`, leaving the footer's
 * reservation uncompensated and adding a dead 30px band under every assistant
 * turn — a doubled gap between consecutive replies.
 *
 * jsdom does no layout, so the class pair is the only observable for this, and
 * it is asserted as an invariant (reserved === compensated) rather than as a
 * literal value: retuning the bar's height stays free, decoupling the halves
 * does not.
 */

const messages: ThreadMessageLike[] = [
  { role: 'user', content: [{ type: 'text', text: 'hello' }] },
  { role: 'assistant', content: [{ type: 'text', text: 'first reply' }] },
  { role: 'assistant', content: [{ type: 'text', text: 'second reply' }] },
];

function Harness() {
  const runtime = useExternalStoreRuntime({
    messages,
    isRunning: false,
    convertMessage: (m: ThreadMessageLike) => m,
    onNew: async () => {},
  });
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <Thread />
    </AssistantRuntimeProvider>
  );
}

/** The `n` in `min-h-<n>` / `-mb-<n>`, or `null` when the utility is absent. */
function spacingStep(classNames: string, prefix: string): string | null {
  const match = classNames
    .split(/\s+/)
    .find(cls => cls.startsWith(`${prefix}-`))
    ?.slice(prefix.length + 1);
  return match ?? null;
}

describe('assistant message action bar spacing', () => {
  it('cancels its reserved height on the same element that reserves it', () => {
    render(<Harness />);

    const footers = screen.getAllByTestId('agent-message').map(root => {
      const footer = root.querySelector('[data-slot="aui_assistant-message-footer"]');
      expect(footer).not.toBeNull();
      return footer as HTMLElement;
    });

    expect(footers.length).toBeGreaterThan(0);

    for (const footer of footers) {
      const reserved = spacingStep(footer.className, 'min-h');
      const compensated = spacingStep(footer.className, '-mb');

      // Reserving height without compensating it on the same element is the
      // regression this test exists for.
      expect(reserved).not.toBeNull();
      expect(compensated).toBe(reserved);
    }
  });
});
