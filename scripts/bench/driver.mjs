#!/usr/bin/env node
/**
 * Load driver for the agent-scale benchmark tier.
 *
 * Fires `openhuman.agent_chat` turns at a running core over /rpc at a fixed
 * concurrency and records per-turn latency and outcome. Writes a JSON summary
 * to --out and, with --turns-out, a JSONL turn log the analyzer correlates
 * against the resource samples.
 *
 * Thread mode is the knob that decides WHICH kind of scale is under test, and
 * the two answer different questions:
 *
 *   fresh       every turn opens a new thread. Models many short, independent
 *               sessions. This is the one that catches per-session state that
 *               is allocated and never reclaimed, because nothing legitimately
 *               accumulates across turns.
 *   per-worker  each worker keeps one thread for the whole run. Models long
 *               conversations. Here RSS growth is EXPECTED — conversation
 *               history genuinely grows — so a rising line is not by itself a
 *               leak, and the analyzer says so.
 *   shared      all workers share a single thread. Contention-focused; the
 *               same caveat about expected growth applies.
 *
 * Usage:
 *   node scripts/bench/driver.mjs --core-url http://127.0.0.1:17788 \
 *     --concurrency 8 --turns 400 --out summary.json --turns-out turns.jsonl
 */

import fs from 'node:fs';

function parseArgs(argv) {
  const opts = {
    coreUrl: 'http://127.0.0.1:17788',
    token: process.env.OPENHUMAN_CORE_TOKEN ?? '',
    concurrency: 4,
    turns: 100,
    durationMs: null,
    threadMode: 'fresh',
    message: 'Summarize the benchmark probe in one sentence.',
    timeoutMs: 120_000,
    out: null,
    turnsOut: null,
    seedSession: true,
    warmupTurns: 0,
  };
  const spec = {
    '--core-url': ['coreUrl', String],
    '--token': ['token', String],
    '--concurrency': ['concurrency', Number],
    '--turns': ['turns', Number],
    '--duration-ms': ['durationMs', Number],
    '--thread-mode': ['threadMode', String],
    '--message': ['message', String],
    '--timeout-ms': ['timeoutMs', Number],
    '--out': ['out', String],
    '--turns-out': ['turnsOut', String],
    '--warmup-turns': ['warmupTurns', Number],
  };
  for (let i = 2; i < argv.length; i += 1) {
    if (argv[i] === '--no-seed-session') {
      opts.seedSession = false;
      continue;
    }
    const entry = spec[argv[i]];
    if (!entry) throw new Error(`unknown argument: ${argv[i]}`);
    const [key, cast] = entry;
    const raw = argv[++i];
    if (raw === undefined) {
      throw new Error(`${argv[i - 1]} expects a value, got nothing`);
    }
    const value = cast(raw);
    if (cast === Number && !Number.isFinite(value)) {
      throw new Error(`${argv[i - 1]} expects a number, got: ${raw}`);
    }
    opts[key] = value;
  }
  if (!['fresh', 'shared', 'per-worker'].includes(opts.threadMode)) {
    throw new Error(`--thread-mode must be fresh|shared|per-worker, got ${opts.threadMode}`);
  }
  if (opts.concurrency < 1) throw new Error('--concurrency must be >= 1');
  if (opts.durationMs === null && opts.turns < 1) {
    throw new Error('need --turns >= 1 or --duration-ms');
  }
  return opts;
}

const opts = parseArgs(process.argv);

async function rpc(method, params, timeoutMs = opts.timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const headers = { 'content-type': 'application/json' };
    if (opts.token) headers.authorization = `Bearer ${opts.token}`;
    const res = await fetch(`${opts.coreUrl}/rpc`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
      signal: controller.signal,
    });
    const text = await res.text();
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${text.slice(0, 400)}`);
    }
    let parsed;
    try {
      parsed = JSON.parse(text);
    } catch {
      throw new Error(`non-JSON response: ${text.slice(0, 400)}`);
    }
    if (parsed.error) {
      const { code, message } = parsed.error;
      throw new Error(`rpc error ${code}: ${message}`);
    }
    return parsed.result;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Install a local session so the managed-inference path can resolve a bearer.
 *
 * A token shaped `<a>.<b>.local` is recognized as a local session and is
 * persisted WITHOUT the `GET /auth/me` round-trip a remote JWT would trigger
 * (see is_local_session_token in src/openhuman/security/credentials/). That is
 * what lets the benchmark skip a real login and keeps the mock's surface to the
 * inference and telemetry routes.
 */
async function seedSession() {
  await rpc('openhuman.auth_store_session', {
    token: 'bench.session.local',
    user: { name: 'agent-scale-bench', email: 'bench@localhost' },
  });
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null;
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[idx];
}

const turnLog = opts.turnsOut ? fs.createWriteStream(opts.turnsOut, { flags: 'w' }) : null;

const results = { ok: 0, failed: 0, latencies: [], errors: new Map() };
let issued = 0;
const startedAt = Date.now();

function shouldContinue() {
  if (opts.durationMs !== null) return Date.now() - startedAt < opts.durationMs;
  return issued < opts.turns;
}

async function runTurn(workerId, threadId, index) {
  const params = { message: `${opts.message} (turn ${index})` };
  if (threadId) params.thread_id = threadId;

  const t0 = performance.now();
  let ok = true;
  let errMessage = null;
  try {
    await rpc('openhuman.agent_chat', params);
  } catch (err) {
    ok = false;
    errMessage = String(err?.message ?? err);
  }
  const latencyMs = performance.now() - t0;

  if (ok) {
    results.ok += 1;
    results.latencies.push(latencyMs);
  } else {
    results.failed += 1;
    // Bucket by message so a summary shows the distinct failure modes rather
    // than one line per failed turn.
    const key = errMessage.slice(0, 200);
    results.errors.set(key, (results.errors.get(key) ?? 0) + 1);
  }

  if (turnLog) {
    turnLog.write(
      `${JSON.stringify({
        tMs: Date.now() - startedAt,
        workerId,
        index,
        latencyMs,
        ok,
        error: errMessage,
      })}\n`,
    );
  }
}

async function worker(workerId) {
  const threadId =
    opts.threadMode === 'per-worker'
      ? `bench-worker-${workerId}`
      : opts.threadMode === 'shared'
        ? 'bench-shared'
        : null;

  while (shouldContinue()) {
    const index = issued++;
    await runTurn(workerId, threadId, index);
  }
}

async function main() {
  if (opts.seedSession) {
    process.stderr.write('[driver] seeding local session\n');
    await seedSession();
  }

  if (opts.warmupTurns > 0) {
    // Warm-up turns fault in lazily-initialized state (model client, stores,
    // tool registry). Measuring them as steady state would show growth that is
    // first-touch initialization, not a leak.
    process.stderr.write(`[driver] warmup: ${opts.warmupTurns} turns\n`);
    for (let i = 0; i < opts.warmupTurns; i += 1) {
      try {
        await rpc('openhuman.agent_chat', { message: `${opts.message} (warmup ${i})` });
      } catch (err) {
        process.stderr.write(`[driver] warmup turn failed: ${err?.message ?? err}\n`);
      }
    }
    // The warm-up must not appear in the measured window.
    results.ok = 0;
    results.failed = 0;
    results.latencies.length = 0;
    results.errors.clear();
    issued = 0;
  }

  const measureStart = Date.now();
  process.stderr.write(
    `[driver] load: concurrency=${opts.concurrency} ` +
      `${opts.durationMs !== null ? `duration=${opts.durationMs}ms` : `turns=${opts.turns}`} ` +
      `thread-mode=${opts.threadMode}\n`,
  );
  await Promise.all(Array.from({ length: opts.concurrency }, (_, i) => worker(i)));
  const wallMs = Date.now() - measureStart;

  const sorted = [...results.latencies].sort((a, b) => a - b);
  const completed = results.ok + results.failed;
  const summary = {
    config: {
      coreUrl: opts.coreUrl,
      concurrency: opts.concurrency,
      turns: opts.turns,
      durationMs: opts.durationMs,
      threadMode: opts.threadMode,
      warmupTurns: opts.warmupTurns,
    },
    measureStartedAtMs: measureStart,
    wallMs,
    turnsCompleted: completed,
    turnsOk: results.ok,
    turnsFailed: results.failed,
    throughputTurnsPerSec: wallMs > 0 ? (completed / wallMs) * 1000 : null,
    latencyMs: {
      min: sorted.length ? sorted[0] : null,
      p50: percentile(sorted, 50),
      p90: percentile(sorted, 90),
      p99: percentile(sorted, 99),
      max: sorted.length ? sorted[sorted.length - 1] : null,
      mean: sorted.length ? sorted.reduce((a, b) => a + b, 0) / sorted.length : null,
    },
    errors: Object.fromEntries(results.errors),
  };

  if (turnLog) await new Promise((resolve) => turnLog.end(resolve));
  const rendered = JSON.stringify(summary, null, 2);
  if (opts.out) fs.writeFileSync(opts.out, rendered);
  process.stdout.write(`${rendered}\n`);

  // A run with no completed turn, or one where every turn failed, produced no
  // usable measurement. Exit non-zero so a caller cannot mistake it for a clean
  // result.
  if (completed === 0 || results.ok === 0) {
    process.stderr.write(`[driver] no usable measurement (completed=${completed}, ok=${results.ok})\n`);
    process.exit(1);
  }
}

main().catch((err) => {
  process.stderr.write(`[driver] fatal: ${err?.stack ?? err}\n`);
  process.exit(1);
});
