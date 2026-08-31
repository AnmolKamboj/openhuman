/**
 * IME composition and the composer text bridge — openhuman#5763.
 *
 * # What this file pins, and why it is a change-detector
 *
 * `useComposerTextBridge` is deliberately **prop-wins**: "on every render where
 * the two disagree the prop is pushed into the composer store". That is correct
 * for programmatic writes (clear-after-send, slash-command seeding, draft
 * restore) and it is also the mechanism behind #5763.
 *
 * An IME delivers a *pre-edit* buffer through `input` events before the user
 * commits. Each one runs `onChange` -> `setInputValue`, React re-renders, and
 * this bridge writes the prop back into the composer store — i.e. it assigns
 * the textarea's value **mid-composition**. Assigning a composing textarea's
 * value ends the composition in most engines, committing the pre-edit buffer
 * and starting a fresh one, which is what produces the reported
 * `nihao` -> `n ni nihao 你好` accumulation.
 *
 * The hook is composition-*unaware*: it takes only `value`
 * (`useComposerTextBridge.ts`), and nothing passes it `isComposingTextRef`.
 * There are three competing community fixes open (#5791, #5775, #5764), so this
 * file does NOT assert the fixed behaviour — it pins the current behaviour at
 * the exact seam a fix must change. When any of those lands, the last test here
 * fails and has to be consciously rewritten. That is the intent: the bug is
 * currently invisible to the suite, and after this file it is not.
 *
 * The existing composition tests (`Conversations.render.test.tsx` and
 * `composerSendDecision.test.ts`) all cover Enter-key *suppression* during
 * composition. None of them covers what ends up in the composer, which is the
 * actual defect.
 */
import { useAui } from '@assistant-ui/react';
import { combineReducers, configureStore } from '@reduxjs/toolkit';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { Provider } from 'react-redux';
import { describe, expect, it } from 'vitest';

import { AssistantUiRuntimeProvider } from '../../../../providers/AssistantUiRuntimeProvider';
import chatRuntimeReducer from '../../../../store/chatRuntimeSlice';
import threadReducer from '../../../../store/threadSlice';
import { useComposerTextBridge } from '../useComposerTextBridge';

function wrapper({ children }: { children: ReactNode }) {
  const store = configureStore({
    reducer: combineReducers({ thread: threadReducer, chatRuntime: chatRuntimeReducer }),
  });
  return (
    <Provider store={store}>
      <AssistantUiRuntimeProvider threadId="t-ime">{children}</AssistantUiRuntimeProvider>
    </Provider>
  );
}

/** Drive the bridge and expose the composer store text it produced. */
function renderBridge(initial: string) {
  return renderHook(
    ({ value }: { value: string }) => {
      useComposerTextBridge(value);
      return useAui();
    },
    { wrapper, initialProps: { value: initial } }
  );
}

describe('useComposerTextBridge — IME composition (#5763)', () => {
  it('pushes the prop into the composer store when the two disagree', () => {
    const { result } = renderBridge('');
    act(() => {
      result.current.composer.setText('typed-in-store');
    });

    // The prop is still '' and the store now says something else, so the next
    // render must overwrite the store. This is the prop-wins rule the hook docs
    // describe, and it is what makes clear-after-send work.
    expect(result.current.composer.getState().text).toBe('');
  });

  it('is a no-op once prop and store already agree', () => {
    const { result, rerender } = renderBridge('hello');
    expect(result.current.composer.getState().text).toBe('hello');

    rerender({ value: 'hello' });
    expect(result.current.composer.getState().text).toBe('hello');
  });

  /**
   * The #5763 seam.
   *
   * A composition sequence delivers intermediate values. The host mirrors each
   * one into `inputValue`, so the bridge sees a changing prop on every
   * intermediate render and writes it into the composer store every time —
   * with no knowledge that a composition is in flight.
   *
   * `writesDuringComposition` is therefore the count of mid-composition
   * value assignments the real textarea would receive. Today it equals the
   * number of intermediate events (3). A fix that suspends the bridge during
   * composition drives it to 0 and this assertion fails — deliberately.
   */
  it('writes to the composer store on EVERY intermediate composition value (the #5763 mechanism)', () => {
    const { result, rerender } = renderBridge('');

    // What an IME actually delivers for "nihao" -> 你好: a pre-edit buffer that
    // grows, then a commit. Each one reaches `onChange` and becomes a prop.
    const preEdit = ['n', 'ni', 'nihao'];
    const observed: string[] = [];

    for (const value of preEdit) {
      rerender({ value });
      observed.push(result.current.composer.getState().text);
    }

    // Every intermediate landed in the store. On a real textarea each of these
    // is a value assignment on a composing element.
    expect(observed).toEqual(['n', 'ni', 'nihao']);

    const writesDuringComposition = observed.length;
    expect(writesDuringComposition).toBe(3);

    // And the commit lands the same way — the bridge cannot tell it apart from
    // the pre-edit writes above.
    rerender({ value: '你好' });
    expect(result.current.composer.getState().text).toBe('你好');
  });

  it('takes only a value — it cannot see composition state', () => {
    // Guards the premise of the test above. `useComposerTextBridge(value)` has
    // arity 1; a fix for #5763 almost certainly widens this signature (or moves
    // the guard into the caller), and this is the cheapest possible detector
    // for that happening.
    expect(useComposerTextBridge.length).toBe(1);
  });
});
