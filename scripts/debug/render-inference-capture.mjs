#!/usr/bin/env node
//
// Render a captured chat-completions request body as a reviewable Markdown
// snapshot, optionally diffed against an earlier capture.
//
// The raw JSON is one long line: unreadable, and impossible to review a prompt
// change in. This lays the same bytes out as prose — envelope, each message,
// each tool schema — and adds the size table that says where the fixed
// per-turn cost actually goes.
//
// Usage:
//   node scripts/debug/render-inference-capture.mjs <after.json> [before.json] > out.md

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const [afterPath, beforePath] = process.argv.slice(2);
if (!afterPath) {
  process.stderr.write('usage: render-inference-capture.mjs <after.json> [before.json]\n');
  process.exit(2);
}

function load(p) {
  const raw = fs.readFileSync(p);
  return { raw, body: JSON.parse(raw.toString('utf8')) };
}

const toolBytes = body => (body.tools || []).reduce((n, t) => n + JSON.stringify(t).length, 0);
const systemBytes = body => (body.messages.find(m => m.role === 'system')?.content || '').length;
// bytes/4 is the same rough token proxy the prompt-budget audit uses; it is a
// consistent yardstick across captures, not a tokenizer.
const tok = b => Math.round(b / 4);

const after = load(afterPath);
const before = beforePath ? load(beforePath) : null;

const out = [];
const w = s => out.push(s);

w('# OpenHuman first inference request');
w('');
w(`> Captured from a real \`openhuman.channel_web_chat\` turn through`);
w('> `scripts/debug/capture-first-inference.mjs`. Authorization headers are not part of');
w('> this artifact.');
w('');

const { body } = after;
const sys = body.messages.find(m => m.role === 'system')?.content || '';

w('## Summary');
w('');
w('| Field | Value |');
w('|---|---|');
w(`| Model | \`${body.model}\` |`);
w(`| Streaming | \`${!!body.stream}\` |`);
w(`| Messages | \`${body.messages.length}\` |`);
w(`| Tools | \`${(body.tools || []).length}\` |`);
w(`| Temperature | \`${body.temperature}\` |`);
w(`| Max tokens | \`${body.max_tokens}\` |`);
if (body.thread_id) w(`| Thread ID | \`${body.thread_id}\` |`);
w(`| Raw-body SHA-256 | \`${crypto.createHash('sha256').update(after.raw).digest('hex')}\` |`);
w('');

w('## Fixed per-turn cost');
w('');
w('Everything below is sent before the user has typed anything.');
w('');
if (before) {
  const rows = [
    ['tools (count)', (before.body.tools || []).length, (body.tools || []).length],
    ['tools (bytes)', toolBytes(before.body), toolBytes(body)],
    ['system prompt (bytes)', systemBytes(before.body), systemBytes(body)],
  ];
  const bTot = toolBytes(before.body) + systemBytes(before.body);
  const aTot = toolBytes(body) + systemBytes(body);
  rows.push(['fixed prefix (bytes)', bTot, aTot]);
  rows.push(['fixed prefix (~tokens)', tok(bTot), tok(aTot)]);
  w(`Compared against \`${path.basename(beforePath)}\`.`);
  w('');
  w('| | before | after | delta |');
  w('|---|---:|---:|---:|');
  for (const [label, b, a] of rows) {
    const d = a - b;
    w(`| ${label} | ${b.toLocaleString()} | ${a.toLocaleString()} | ${d >= 0 ? '+' : ''}${d.toLocaleString()} |`);
  }
  w('');
  w(`Fixed prefix **${(100 * (aTot - bTot) / bTot).toFixed(1)}%**.`);
  w('');
  const bn = new Set((before.body.tools || []).map(t => t.function.name));
  const an = new Set((body.tools || []).map(t => t.function.name));
  const gone = [...bn].filter(n => !an.has(n)).sort();
  const added = [...an].filter(n => !bn.has(n)).sort();
  if (gone.length) w(`**No longer advertised** (${gone.length}): ${gone.map(n => `\`${n}\``).join(', ')}`);
  if (added.length) { w(''); w(`**Newly advertised** (${added.length}): ${added.map(n => `\`${n}\``).join(', ')}`); }
  w('');
} else {
  w('| Part | Bytes | ~tokens |');
  w('|---|---:|---:|');
  w(`| system prompt | ${systemBytes(body).toLocaleString()} | ${tok(systemBytes(body)).toLocaleString()} |`);
  w(`| tools array | ${toolBytes(body).toLocaleString()} | ${tok(toolBytes(body)).toLocaleString()} |`);
  w(`| **total** | **${(systemBytes(body) + toolBytes(body)).toLocaleString()}** | **${tok(systemBytes(body) + toolBytes(body)).toLocaleString()}** |`);
  w('');
}

w('### System-prompt sections, largest first');
w('');
w('| Bytes | Section |');
w('|---:|---|');
for (const part of sys.split(/\n(?=#{1,3} )/).sort((a, b) => b.length - a.length).slice(0, 15)) {
  w(`| ${part.length.toLocaleString()} | ${part.split('\n')[0].replace(/\|/g, '\\|')} |`);
}
w('');

w('### Tool schemas, largest first');
w('');
w('| Bytes | Tool |');
w('|---:|---|');
for (const t of [...(body.tools || [])].sort((x, y) => JSON.stringify(y).length - JSON.stringify(x).length)) {
  w(`| ${JSON.stringify(t).length.toLocaleString()} | \`${t.function.name}\` |`);
}
w('');

w('## Request envelope');
w('');
w('````json');
const env = { ...body };
delete env.messages;
delete env.tools;
w(JSON.stringify(env));
w('````');
w('');

w('# Messages');
w('');
body.messages.forEach((m, i) => {
  w(`## Message ${i + 1} — \`${m.role}\``);
  w('');
  const content = typeof m.content === 'string' ? m.content : JSON.stringify(m.content, null, 2);
  if (m.role === 'system') {
    w('<details open>');
    w('<summary>Full system prompt</summary>');
    w('');
    w(content);
    w('');
    w('</details>');
  } else {
    w(content);
  }
  w('');
  w('---');
  w('');
});

w('# Tools');
w('');
w(`Sent as the request's top-level \`tools\` array alongside the system prompt, not after the`);
w('messages — native function calling. The order here is the order on the wire.');
w('');
(body.tools || []).forEach((t, i) => {
  w(`### ${i + 1}. \`${t.function.name}\``);
  w('');
  w(t.function.description || '_(no description)_');
  w('');
  // Minified, matching what the provider actually receives. Pretty-printing
  // costs roughly a third more lines for indentation the reader gains nothing
  // from, and it misrepresents the byte count the tables above report.
  w('````json');
  w(JSON.stringify(t.function.parameters));
  w('````');
  w('');
});

process.stdout.write(out.join('\n'));
