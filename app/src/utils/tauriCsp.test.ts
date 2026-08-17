import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const configPath = fileURLToPath(new URL('../../src-tauri/tauri.conf.json', import.meta.url));
const config = JSON.parse(readFileSync(configPath, 'utf8')) as {
  app?: { security?: { csp?: string } };
};

describe('Tauri content security policy', () => {
  it('allows scripts served from the Wry custom scheme', () => {
    const scriptSource = config.app?.security?.csp
      ?.split(';')
      .map(directive => directive.trim())
      .find(directive => directive.startsWith('script-src '));

    expect(scriptSource).toBeDefined();
    expect(scriptSource).toContain('tauri:');
    expect(scriptSource).toContain('tauri://localhost');
  });

  it('retains the existing script execution requirements', () => {
    const scriptSource = config.app?.security?.csp
      ?.split(';')
      .map(directive => directive.trim())
      .find(directive => directive.startsWith('script-src '));

    expect(scriptSource).toContain("'self'");
    expect(scriptSource).toContain("'wasm-unsafe-eval'");
  });
});
