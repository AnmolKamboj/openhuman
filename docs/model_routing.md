# Which model for which OpenHuman task

Settings → Inference. Do not change a locked row without discussion.

| Task | Use it for | Use this model |
|---|---|---|
| **Chat** | Quick mode. Direct back-and-forth. | **OpenAI · gpt-5.6-luna** |
| **Reasoning** | Reasoning mode. Main agent, meeting summaries, heavy synthesis. | **Cursor · grok-4.6** (`effort:high`, `fast:true`) |
| **Agentic** | Sub-agents, tool loops, GIF decisions. | **Cursor · composer-2.5** (`fast:false`) |
| **Coding** | Code generation and refactors. | **Cursor · grok-4.6** (`effort:high`, `fast:false`) |
| **Vision** | Image understanding. Must be multimodal. | **OpenAI · gpt-5.6-luna** |
| **Memory summarization** | Tree extracts and consolidations. | **OpenAI · gpt-5.6-luna** |
| **Heartbeat** | Background reasoning between turns. | **OpenAI · gpt-5.6-luna** |
| **Learning · Reflections** | Periodic reflection over recent history. | **NVIDIA · z-ai/glm-5.2** |
| **Subconscious** | Eventfulness scoring and drift checks. | **OpenAI · gpt-5.6-luna** |

Chat, Vision, Memory, Heartbeat, and Subconscious all use Luna: cheap, fast, good enough for compact text. Learning stays on GLM because reflections need more synthesis. Reasoning is Grok **fast**. Coding is Grok **not fast**.

---

## Embeddings — Custom (OpenAI-compatible)

Settings → Embeddings. The tree **hard-requires 1024**. Do not use the form defaults (`text-embedding-3-small`) unless you have an OpenAI key.

| Field | Value |
|---|---|
| Custom endpoint | `https://integrate.api.nvidia.com/v1` |
| Model name | `nvidia/nv-embedqa-e5-v5` |
| Dimensions | `1024` |
| API key | NVIDIA key from `.env` (`NVIDIA_API_KEY`). Same key as GLM. |

Then **Test connection**. This fork's NVIDIA embedder sends `input_type` (`query` / `passage`). Do not use `nvidia/nemotron-3-embed-1b` (2048-only, HTTP 400 at 1024).
