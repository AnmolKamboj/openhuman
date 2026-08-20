# Agent-scale benchmark — findings

Measured with `scripts/bench/run-agent-scale.sh` against a release
`openhuman-core` on one Linux box, mocked LLM, concurrency 8, `fresh` thread
mode. Every number below is reproducible with the commands shown.

## Summary

Under sustained agentic load the core degrades: per-turn latency climbs
linearly, throughput falls to about a third of its starting rate, and RSS grows
without plateauing.

**It is not a memory leak, and it is not a missing index.** Memory recall loads
the *entire* memory namespace — every document and every vector chunk — on every
turn, decodes each embedding, and scores them in-process behind a single
connection mutex. Cost is Θ(memories stored) per turn, so a session's total work
is quadratic and the throughput ceiling falls as memories accumulate. RSS grows
because the working set is the stored data.

## Evidence

### 1. It tracks stored data, not process uptime

A fresh core process pointed at an already-populated workspace **inherits the
full cost immediately** rather than starting fast:

| Run | Workspace | Start latency | End latency |
| --- | --- | --- | --- |
| C1 | empty | 111 ms | 438 ms |
| C2 | C1's, **fresh process** | **532 ms** | 534 ms |

```bash
scripts/bench/run-agent-scale.sh --duration-ms 240000 --keep-workspace --out-dir target/bench/C1
scripts/bench/run-agent-scale.sh --duration-ms 120000 --workspace target/bench/C1/workspace --out-dir target/bench/C2
```

A leak would have been left behind with the old process. This rules that out.

### 2. Turning the memory subsystem off removes every symptom

```bash
scripts/bench/run-agent-scale.sh --duration-ms 240000 --tool-depth 0            # memory on
scripts/bench/run-agent-scale.sh --duration-ms 240000 --tool-depth 0 --memory-off
```

| | memory ON | memory OFF |
| --- | --- | --- |
| latency over 4 min | 111 → 438 ms | **flat ~79 ms** |
| throughput retained | 33% | **102%** |
| RSS growth | +109 KiB/turn | **−0.65 KiB/turn** |
| CPU per turn | 253 ms | **28 ms** |
| turns completed | ~8,600 | **~24,500** |

Nothing else in the turn path shows the behaviour. Tool depth is irrelevant —
`--tool-depth 0` (no `memory_search` calls at all) degrades identically, so this
is the implicit per-turn recall, not the agent's memory tool.

### 3. What the recall actually does

At 8,600 turns the store held 9,660 documents and 10,127 chunks, all in one
`global` namespace. Per recall it loads all of both:

```
chunks : 10127 rows, 39.6 MiB of embeddings materialized
docs   : 9660 rows
        ~61 ms of SQL per recall, warm cache
```

The indexes exist and are used (`idx_vector_chunks_ns_doc`,
`idx_memory_docs_ns_updated` — `SEARCH … USING INDEX`). They cannot help: the
query has no selective predicate, it wants every row in the namespace.
`query_namespace_hits_excluding_session` does take a `limit`, but applies it
after loading and scoring everything.

### 4. It is the read path, not write contention

Reads and writes share one connection, and `--memory-off` disables both — so on
its own it cannot say which is expensive. Against an already-populated
workspace, with writes off but recall still scanning every turn:

```bash
scripts/bench/run-agent-scale.sh --duration-ms 90000 --tool-depth 0 \
  --memory-writes-off --workspace <populated>
```

| | throughput | p50 |
| --- | --- | --- |
| reads + writes | 14.6/s | 540 ms |
| **reads only** | **15.8/s** | **499 ms** |

Removing every write buys ~8%. The recall scan is the cost; write contention is
a rounding error.

### 5. The single connection mutex sets the ceiling

`UnifiedMemory` owns one `Mutex<Connection>`, so the scan serializes across
concurrent turns. Throughput saturates accordingly:

| concurrency | p50 | throughput |
| --- | --- | --- |
| 1 | 212 ms | 4.7/s |
| 2 | 239 ms | 8.2/s |
| 8 | 550 ms | 14.5/s |

Saturation at ~14.5/s implies ~65 ms per turn in a serialized section, matching
the measured ~61 ms scan. Adding cores cannot raise this, and it falls as the
namespace grows.

## A fix that was tried and reverted — do not repeat it

**Hypothesis:** `load_chunks_for_scope` decoded every embedding blob into a
`Vec<f32>` *inside* the connection lock — ~10k allocations and ~10M
little-endian conversions per recall at this scale, with every concurrent turn
blocked behind it. Moving the decode outside the lock should shorten the
critical section and raise the ceiling.

**Result: no measurable improvement.** Interleaved A/B, two reps each, identical
populated workspaces, concurrency 8, 90 s:

| arm | rep 1 | rep 2 | mean |
| --- | --- | --- | --- |
| baseline | 14.87/s | 14.37/s | 14.62/s |
| decode outside lock | 14.42/s | 14.18/s | 14.30/s |

The point estimate is slightly *negative* and well inside the ~3% within-arm
spread. The extra intermediate vector plausibly costs as much as the decode
saved. The change was reverted; `vendor/tinymemory` is untouched.

**What that rules out:** the decode is not the serialized cost. Combined with
finding 4 (writes are ~8%), the critical section is the SQLite scan itself —
iterating 10k rows and copying ~40 MiB of blobs through one mutex, per turn.
Any fix has to avoid *reading* the whole namespace. Shaving work around the scan
does not help.

## Not fixed — needs a design decision

Making recall sub-linear means making it selective, and every option changes
retrieval behaviour, so it is the memory owners' call rather than a benchmark
fix:

- **Bound the candidate set** (recency window). Cheapest; old memories become
  unreachable by vector search.
- **Two-stage retrieval** — FTS/keyword prefilter, then vector-rank the
  survivors. `episodic_fts` already exists. Changes which memories surface.
- **A real vector index** (sqlite-vec, HNSW). Correct long-term answer, largest
  change.
- **Concurrent readers.** Independent of the above, and the only option here
  that changes no behaviour: SQLite WAL supports concurrent readers, but one
  mutex-guarded connection serializes them. A read-only connection pool would
  let recalls proceed in parallel. Note this raises the ceiling without removing
  the per-turn O(N) work, so it defers the problem rather than solving it —
  but unlike the others it cannot change which memories surface.

A deliberately-bounded recall would also make the memory verdict in this harness
unambiguous, since RSS would stop tracking stored data.

## Separate finding — journal write amplification

Every agent run writes a ~604 KB journal file to `tinyagents_store/journal/`,
for a single trivial turn. One line accounts for ~424 KB: the model-call event
serializes the full system prompt and the complete tool schemas for ~80 tools.
A 4-minute run left ~5 GB behind; with memory off (so more turns complete) it
reached ~13 GB.

This is O(1) per turn, so it is **not** the cause of the degradation above, and
the benchmark does not fail on it. It is flagged because ~600 KB per turn of
mostly-static text is a real cost for long-lived installs.
