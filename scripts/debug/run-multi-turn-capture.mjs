#!/usr/bin/env node
/**
 * Drive a multi-turn chat session over the production RPC
 * (`openhuman.channel_web_chat` — the same call the desktop composer makes) so a
 * capture proxy running with `CAPTURE_ALL=1` records the whole sequence.
 *
 * Single-turn captures cannot see a prefix-cache regression: the question is not
 * what turn 1 costs, it is whether turn 2 can reuse it. Pair with
 * `audit-inference-prefix.mjs`.
 *
 * Usage:
 *   node scripts/debug/run-multi-turn-capture.mjs [--core-url URL] [--thread-id ID]
 *
 * The thread id must be unrelated to any earlier run's: the session key is the
 * agent name plus a TRUNCATED thread id, so two ids sharing a prefix resume the
 * same stored session and replay its frozen system prompt.
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};

const coreUrl = flag('--core-url', process.env.OPENHUMAN_CORE_RPC_URL || 'http://127.0.0.1:7799/rpc');
const threadId = flag('--thread-id', `pfx-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`);
const token =
  process.env.OPENHUMAN_CORE_TOKEN ||
  fs.readFileSync(path.join(os.homedir(), '.openhuman', 'core.token'), 'utf8').trim();

// Turn 1 is plain chat; turn 2 forces a tool round so the replayed history
// contains an assistant-with-tool_calls / tool-result pair; turn 3 replays both.
const TURNS = [
  'heya',
  'What is the exact current date and time right now? Use a tool to check, do not guess.',
  'Thanks. In one short sentence, what did you just tell me?',
];

async function rpc(method, params) {
  const response = await fetch(coreUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body: JSON.stringify({ jsonrpc: '2.0', id: `mtc-${Date.now()}`, method, params }),
    signal: AbortSignal.timeout(300_000),
  });
  const text = await response.text();
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    throw new Error(`non-JSON RPC response (${response.status}): ${text.slice(0, 300)}`);
  }
  if (body.error) throw new Error(`RPC ${method} failed: ${JSON.stringify(body.error).slice(0, 400)}`);
  return body.result;
}

console.log(`[multi-turn] core=${coreUrl} thread_id=${threadId}`);

for (const [index, message] of TURNS.entries()) {
  const started = Date.now();
  const result = await rpc('openhuman.channel_web_chat', {
    client_id: `prefix-audit-${threadId}`,
    thread_id: threadId,
    message,
  });
  const reply = typeof result === 'string' ? result : JSON.stringify(result ?? '');
  console.log(
    `[multi-turn] turn ${index + 1}/${TURNS.length} ok in ${Date.now() - started}ms — ` +
      `reply ${reply.length} chars`
  );
}

console.log(`[multi-turn] done; thread_id=${threadId}`);
