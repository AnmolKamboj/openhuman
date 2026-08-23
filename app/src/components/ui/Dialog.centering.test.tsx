/**
 * Regression: a centered dialog opened offset toward the bottom-right and
 * snapped into place when its entrance animation finished.
 *
 * `DialogContent` / `AlertDialogContent` center with `fixed left-1/2 top-1/2`
 * plus `-translate-x-1/2 -translate-y-1/2`. They used `animate-fade-up`, whose
 * keyframes set `transform: translateY(…)`. An animated property replaces the
 * same property from a normal declaration for the animation's whole duration,
 * so the centering transform was dropped while the animation ran.
 *
 * Asserting the class name alone would only catch a literal revert. These tests
 * assert the RULE instead, against the real Tailwind v4 theme: any entrance
 * animation used by a transform-centered element must carry the centering in
 * every one of its own keyframes, and must land exactly where the utility
 * classes leave it so nothing moves when the animation hands the element back.
 */
import { render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

import AlertDialogRoot, { AlertDialogContent } from './AlertDialog';
import { DialogContent, DialogRoot } from './Dialog';

/** The animation registrations and keyframes, read from the real theme. */
const tailwindSource = readFileSync(
  join(__dirname, '..', '..', 'index.css'),
  'utf8'
);

/** `--animate-dialog-in: dialogIn 0.2s …` → the keyframe name `dialogIn`. */
const keyframeNameFor = (animation: string): string => {
  const match = tailwindSource.match(new RegExp(`--animate-${animation}:\\s*(\\w+)`));
  if (!match) throw new Error(`no animation registered as '${animation}'`);
  return match[1];
};

/** Every `transform:` value inside the named keyframe block. */
const transformsIn = (keyframe: string): string[] => {
  const start = tailwindSource.indexOf(`  @keyframes ${keyframe} {`);
  if (start === -1) throw new Error(`no keyframes named ${keyframe}`);
  const end = tailwindSource.indexOf('\n  }', start);
  const block = tailwindSource.slice(start, end);
  return Array.from(block.matchAll(/transform:\s*([^;]+);/g)).map(m => m[1].trim());
};

const centeringClassesOf = (el: HTMLElement) => ({
  centersX: el.className.includes('-translate-x-1/2'),
  centersY: el.className.includes('-translate-y-1/2'),
  animation: /animate-([\w-]+)/.exec(el.className)?.[1],
});

describe('centered dialog entrance animation', () => {
  it.each([
    [
      'DialogContent',
      () => (
        <DialogRoot open>
          <DialogContent aria-label="d">body</DialogContent>
        </DialogRoot>
      ),
    ],
    [
      'AlertDialogContent',
      () => (
        <AlertDialogRoot open>
          <AlertDialogContent aria-label="d">body</AlertDialogContent>
        </AlertDialogRoot>
      ),
    ],
  ])('%s keeps its centering transform for the whole animation', (_name, renderIt) => {
    render(renderIt());
    const content = screen.getByText('body').closest('[data-slot$="dialog-content"]');
    expect(content).not.toBeNull();

    const { centersX, centersY, animation } = centeringClassesOf(content as HTMLElement);
    // Guard the premise: if it ever stops centering by transform, this whole
    // rule is moot and the test should be revisited rather than silently pass.
    expect(centersX && centersY).toBe(true);
    expect(animation).toBeDefined();

    const transforms = transformsIn(keyframeNameFor(animation!));
    // An entrance that animates no transform at all cannot clobber the
    // centering, and is equally correct.
    for (const transform of transforms) {
      expect(transform, `keyframe of ${animation} drops the centering offset`).toMatch(
        /translate\(\s*-50%/
      );
      expect(transform, `keyframe of ${animation} drops the centering offset`).toMatch(/-50%/);
    }
  });

  it('lands exactly on the resting transform, so nothing moves when it ends', () => {
    const transforms = transformsIn(keyframeNameFor('dialog-in'));
    // The utility pair resolves to translate(-50%, -50%); the final frame must
    // match it or the dialog jumps as the animation releases the element.
    expect(transforms.at(-1)).toBe('translate(-50%, -50%)');
  });
});
