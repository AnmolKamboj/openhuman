import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import { cn } from './cn';

/**
 * `tailwind.config.js` is CommonJS in a `"type": "module"` package, and it
 * calls `require()` for its plugins — so neither a plain import nor
 * `createRequire` loads it under Vite. Evaluate it in a sandbox that stubs
 * `module`/`require` instead, which is the same "read the config as data"
 * approach `flows/canvas/__tests__/flowCanvasOutlines.test.ts` takes with
 * `index.css`.
 */
function loadTailwindConfig(): { theme: { extend: Record<string, Record<string, unknown>> } } {
  // Vitest runs with `app/` as cwd (see test/vitest.config.ts).
  const configPath = resolve(process.cwd(), 'tailwind.config.js');
  const source = readFileSync(configPath, 'utf8');
  const sandboxedModule = { exports: {} as Record<string, unknown> };
  const evaluate = new Function(
    'module',
    'exports',
    'require',
    `${source}\nreturn module.exports;`
  ) as (
    module: typeof sandboxedModule,
    exports: Record<string, unknown>,
    require: () => unknown
  ) => { theme: { extend: Record<string, Record<string, unknown>> } };

  // Plugins are irrelevant here — only `theme.extend` is under test.
  return evaluate(sandboxedModule, sandboxedModule.exports, () => ({}));
}

const tailwindConfig = loadTailwindConfig();

describe('cn', () => {
  it('flattens conditionals like clsx', () => {
    // Held in a variable rather than written inline: `false && 'b'` is a
    // constant expression, which `no-constant-binary-expression` rejects.
    const isActive = false;
    expect(cn('a', isActive && 'b', undefined, ['c', 'd'])).toBe('a c d');
  });

  it('resolves conflicts last-wins so a caller className beats the default', () => {
    expect(cn('bg-primary-500', 'bg-coral-500')).toBe('bg-coral-500');
    expect(cn('bg-surface/50', 'bg-surface')).toBe('bg-surface');
  });

  // The three groups below are the dangerous ones: without the extendTailwindMerge
  // config, tailwind-merge classifies each custom key as a COLOUR, which is a
  // different group, so it gets silently dropped rather than harmlessly kept.
  it('keeps text-micro as a font size, not a text colour', () => {
    const out = cn('text-micro', 'text-content');
    expect(out).toContain('text-micro');
    expect(out).toContain('text-content');
  });

  it('treats the custom shadows as shadows, so a later shadow-sm replaces them', () => {
    expect(cn('shadow-content-edge', 'shadow-xl')).toBe('shadow-xl');
    expect(cn('shadow-cmd-palette', 'shadow-soft')).toBe('shadow-soft');
  });

  it('treats the custom background images as bg-image, not bg-colour', () => {
    const out = cn('bg-noise', 'bg-surface');
    expect(out).toContain('bg-noise');
    expect(out).toContain('bg-surface');
  });

  it('resolves the custom radius scale', () => {
    expect(cn('rounded-xs', 'rounded-5xl')).toBe('rounded-5xl');
  });

  it('resolves the custom backdrop blur-sm scale', () => {
    expect(cn('backdrop-blur-xs', 'backdrop-blur-3xl')).toBe('backdrop-blur-3xl');
  });
});

/**
 * The drift guard. Every custom scale key in `tailwind.config.js` for the
 * groups tailwind-merge would otherwise misclassify must be registered in
 * `cn.ts`. Adding `rounded-6xl` to the config and not to `cn.ts` fails here
 * rather than silently producing a component whose radius never applies.
 */
describe('cn stays in sync with tailwind.config.js', () => {
  const STOCK = {
    fontSize: ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl', '7xl'],
    borderRadius: ['none', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', 'full', 'DEFAULT'],
    boxShadow: ['sm', 'md', 'lg', 'xl', '2xl', 'inner', 'none', 'DEFAULT'],
    backgroundImage: ['none'],
    backdropBlur: ['none', 'sm', 'md', 'lg', 'xl'],
  } as const;

  // Keys the cn.ts config registers, per group.
  const REGISTERED = {
    fontSize: ['micro'],
    borderRadius: ['xs', '4xl', '5xl'],
    boxShadow: [
      'glow',
      'glow-lg',
      'inner-glow',
      'subtle',
      'soft',
      'medium',
      'large',
      'float',
      'crisp',
      'cmd-palette',
      'content-edge',
    ],
    backgroundImage: ['noise', 'gradient-mesh', 'gradient-radial', 'gradient-conic'],
    backdropBlur: ['xs', '2xl', '3xl'],
  } as const;

  for (const group of Object.keys(REGISTERED) as (keyof typeof REGISTERED)[]) {
    it(`registers every non-stock ${group} key`, () => {
      const configured = Object.keys(tailwindConfig.theme.extend[group] ?? {});
      const needsRegistering = configured.filter(
        key => !(STOCK[group] as readonly string[]).includes(key)
      );
      const missing = needsRegistering.filter(
        key => !(REGISTERED[group] as readonly string[]).includes(key)
      );
      expect(missing, `add these ${group} keys to the classGroups in src/lib/cn.ts`).toEqual([]);
    });
  }
});
