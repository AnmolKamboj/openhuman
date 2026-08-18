import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const configPath = fileURLToPath(new URL('../../src-tauri/tauri.conf.json', import.meta.url));
const config = JSON.parse(readFileSync(configPath, 'utf8')) as {
  app?: { security?: { csp?: string } };
};
const scriptSourceTokens =
  config.app?.security?.csp
    ?.split(';')
    .map(directive => directive.trim())
    .find(directive => directive.startsWith('script-src '))
    ?.split(/\s+/)
    .slice(1) ?? [];

describe('Tauri content security policy', () => {
  it('allows scripts served from the Wry custom scheme', () => {
    expect(scriptSourceTokens).toEqual(expect.arrayContaining(['tauri:', 'tauri://localhost']));
  });

  it('retains the existing script execution requirements', () => {
    expect(scriptSourceTokens).toEqual(
      expect.arrayContaining(["'self'", "'wasm-unsafe-eval'", 'https://www.googletagmanager.com'])
    );
  });
});
