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

// `channel_web_chat` ACKs immediately and runs the turn asynchronously, streaming
// over the socket. Firing the next turn on the ACK would put several turns into
// one thread concurrently — which is not a multi-turn session at all, and makes
// the capture meaningless: each overlapping turn starts from an empty history, so
// every request looks like turn 1 and the sequence appears never to accumulate.
//
// Waiting for the *capture directory* to go quiet is not enough either, and that
// is a mistake worth recording: the inference request is sent at the START of the
// turn, so a long streamed reply is still being generated and persisted long
// after the last request landed. It produced exactly the false reading above.
//
// The turn's own transcript is the completion signal — it is written when the
// turn ends — so wait for this thread's persisted assistant-message count to
// rise.
const captureDir = path.resolve(
  process.env.CAPTURE_ALL_DIR || 'target/debug-logs/inference-sequence'
);

const capturedCount = () =>
  fs.existsSync(captureDir) ? fs.readdirSync(captureDir).filter(f => f.endsWith('.json')).length : 0;

const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

/**
 * Assistant messages persisted for this thread, across every transcript file
 * that belongs to it (a resume mints a new file, so one thread can own several).
 */
function persistedReplies() {
  const roots = fs.existsSync(path.join(os.homedir(), '.openhuman', 'users'))
    ? fs.readdirSync(path.join(os.homedir(), '.openhuman', 'users'))
    : [];
  // The session key truncates the thread id, so match on a prefix rather than
  // the whole thing.
  const key = threadId.slice(0, 12);
  let replies = 0;
  for (const user of roots) {
    const dir = path.join(os.homedir(), '.openhuman', 'users', user, 'workspace', 'session_raw');
    if (!fs.existsSync(dir)) continue;
    for (const file of fs.readdirSync(dir)) {
      if (!file.includes(key) || !file.endsWith('.jsonl')) continue;
      for (const line of fs.readFileSync(path.join(dir, file), 'utf8').split('\n')) {
        if (!line.trim()) continue;
        try {
          if (JSON.parse(line).role === 'assistant') replies += 1;
        } catch {
          /* a partially flushed line — it will be counted on the next poll */
        }
      }
    }
  }
  return replies;
}

/** Wait until this thread has persisted one more assistant reply than before. */
async function waitForTurnToSettle(repliesBefore, { timeoutMs = 300_000 } = {}) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await sleep(1_000);
    if (persistedReplies() > repliesBefore) {
      // The transcript is written at the end of the turn; give the rest of the
      // turn's teardown a beat before the next one starts.
      await sleep(2_000);
      return;
    }
  }
  throw new Error(`turn did not persist a reply within ${timeoutMs}ms`);
}

console.log(`[multi-turn] core=${coreUrl} thread_id=${threadId}`);

for (const [index, message] of TURNS.entries()) {
  const before = capturedCount();
  const started = Date.now();
  await rpc('openhuman.channel_web_chat', {
    client_id: `prefix-audit-${threadId}`,
    thread_id: threadId,
    message,
  });
  const after = await waitForTurnToSettle(before);
  console.log(
    `[multi-turn] turn ${index + 1}/${TURNS.length} settled in ${Date.now() - started}ms — ` +
      `${after - before} inference request(s)`
  );
}

console.log(`[multi-turn] done; thread_id=${threadId}, ${capturedCount()} request(s) captured`);
