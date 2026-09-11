#!/usr/bin/env node
//
// Convert a captured chat-completions request body to Markdown, verbatim.
//
// This is a format change and nothing else: every byte the model receives is
// reproduced, in wire order, with no summary, no size accounting and no
// commentary. The point is to read exactly what was sent — an analysis layer
// on top would be a second thing to keep honest, and the raw JSON is one
// unreadable line, which is the only problem this solves.
//
// Usage:
//   node scripts/debug/render-inference-capture.mjs <capture.json> > out.md

import fs from 'node:fs';

const [capturePath] = process.argv.slice(2);
if (!capturePath) {
  process.stderr.write('usage: render-inference-capture.mjs <capture.json>\n');
  process.exit(2);
}

const body = JSON.parse(fs.readFileSync(capturePath, 'utf8'));
const out = [];
const w = s => out.push(s);

// Everything that is not `messages` or `tools`, kept minified: it is one short
// object and pretty-printing it would misrepresent the bytes on the wire.
const envelope = { ...body };
delete envelope.messages;
delete envelope.tools;

w('# Request');
w('');
w('```json');
w(JSON.stringify(envelope));
w('```');
w('');

w('# Messages');
w('');
(body.messages || []).forEach((m, i) => {
  w(`## ${i + 1}. \`${m.role}\``);
  w('');
  // String content is reproduced as-is — it is already Markdown and rewriting
  // it would defeat the purpose. Anything structured (multimodal parts, tool
  // calls) is shown as the JSON it is.
  if (typeof m.content === 'string') {
    w(m.content);
  } else {
    w('```json');
    w(JSON.stringify(m.content));
    w('```');
  }
  if (m.tool_calls) {
    w('');
    w('```json');
    w(JSON.stringify(m.tool_calls));
    w('```');
  }
  w('');
});

w('# Tools');
w('');
(body.tools || []).forEach((t, i) => {
  const f = t.function || t;
  w(`## ${i + 1}. \`${f.name}\``);
  w('');
  if (f.description) {
    w(f.description);
    w('');
  }
  w('```json');
  w(JSON.stringify(f.parameters ?? {}));
  w('```');
  w('');
});

process.stdout.write(out.join('\n'));
