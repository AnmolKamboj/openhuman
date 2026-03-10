# Gmail Skill Integration — Design Reference

**Scope:** Native Gmail skill — OAuth2 connect flow, tunnel-based callback, token storage,
initial inbox sync, and `send_email` tool dispatch.

**Status:** Design reference / implementation guide.

---

## 1. Overview

This document describes the full lifecycle of connecting a Gmail skill to ZeroClaw.
It covers four concerns in order:

1. **OAuth2 connect** — how the user authorises Gmail access
2. **Tunnel callback** — how the authorization code reaches the backend through the tunnel
3. **Token storage** — how credentials are encrypted and persisted in the auth profile store
4. **Email sync + send** — how the initial inbox is fetched and how sending works

The skill is declared with the following manifest:

```json
{
  "id": "gmail",
  "name": "Gmail",
  "version": "1.0.0",
  "description": "Gmail integration via Google API — comprehensive email management with OAuth2 authentication, send/receive, labels, search, and attachments.",
  "auto_start": false,
  "platforms": ["windows", "macos", "linux", "android", "ios"],
  "setup": {
    "required": true,
    "label": "Connect Gmail",
    "oauth": {
      "provider": "gmail",
      "scopes": [
        "https://www.googleapis.com/auth/gmail.send",
        "https://www.googleapis.com/auth/gmail.modify",
        "https://www.googleapis.com/auth/gmail.labels"
      ]
    }
  }
}
```

The equivalent `SKILL.toml` that ZeroClaw loads from
`~/.zeroclaw/workspace/skills/gmail/SKILL.toml` is shown in §2.

---

## 2. Skill Manifest (`SKILL.toml`)

Skills in ZeroClaw are loaded by `src/skills/mod.rs`. The loader reads a `SKILL.toml`
(or `SKILL.md`) file from `~/.zeroclaw/workspace/skills/<name>/`. The Rust struct it
populates is `Skill` / `SkillManifest` / `SkillTool`.

`SKILL.toml` for Gmail:

```toml
[skill]
name        = "gmail"
version     = "1.0.0"
description = "Gmail integration via Google API — send/receive, labels, search, attachments."
author      = "zeroclaw"
tags        = ["email", "google", "productivity"]

# OAuth setup block — interpreted by the skill connect subsystem (see §3).
[skill.setup]
required = true
label    = "Connect Gmail"

[skill.setup.oauth]
provider = "gmail"
scopes   = [
  "https://www.googleapis.com/auth/gmail.send",
  "https://www.googleapis.com/auth/gmail.modify",
  "https://www.googleapis.com/auth/gmail.labels",
]

# Tools exposed to the agent after connection.

[[tools]]
name        = "gmail_send_email"
description = "Send an email via Gmail. Requires a connected Gmail account."
kind        = "http"
command     = "https://gmail.googleapis.com/gmail/v1/users/me/messages/send"

[[tools]]
name        = "gmail_list_messages"
description = "List messages in the Gmail inbox. Supports query filters."
kind        = "http"
command     = "https://gmail.googleapis.com/gmail/v1/users/me/messages"

[[tools]]
name        = "gmail_get_message"
description = "Fetch the full content of a single Gmail message by ID."
kind        = "http"
command     = "https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}"

[[tools]]
name        = "gmail_modify_labels"
description = "Add or remove labels on a Gmail message."
kind        = "http"
command     = "https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}/modify"
```

At runtime, `load_skills_with_config()` (`src/skills/mod.rs:78`) reads these tools into
`Vec<SkillTool>` and injects them into the agent system prompt. The `kind = "http"` tools
are dispatched via `HttpRequestTool` (`src/tools/http_request.rs`) with the stored
Bearer token injected as the `Authorization` header (see §5).

---

## 3. OAuth2 Connect Flow

### 3.1 What the user triggers

When the user asks to connect Gmail (via the frontend or a CLI command), the backend:

1. Generates a **PKCE pair** (code verifier + SHA-256 challenge) — following the same
   pattern as `src/auth/openai_oauth.rs:generate_pkce_state()`.
2. Generates a random CSRF `state` token (24 bytes, base64url).
3. Constructs the Google OAuth2 authorisation URL:

```
https://accounts.google.com/o/oauth2/v2/auth
  ?response_type=code
  &client_id=<GOOGLE_CLIENT_ID>
  &redirect_uri=<TUNNEL_PUBLIC_URL>/auth/callback/gmail
  &scope=https://www.googleapis.com/auth/gmail.send
         https://www.googleapis.com/auth/gmail.modify
         https://www.googleapis.com/auth/gmail.labels
  &code_challenge=<SHA256_CHALLENGE>
  &code_challenge_method=S256
  &state=<RANDOM_STATE>
  &access_type=offline
  &prompt=consent
```

`access_type=offline` is required to receive a `refresh_token` from Google.
`prompt=consent` ensures the refresh token is issued on every connect, not just the first.

4. The URL is sent to the frontend. The frontend opens it in a browser or WebView.

### 3.2 Where the redirect URI points

The `redirect_uri` is the **tunnel public URL** with the path `/auth/callback/gmail`.

ZeroClaw's tunnel system (`src/tunnel/mod.rs`) exposes the local gateway port via one of:

| Provider     | How public URL is obtained                                       |
| ------------ | ---------------------------------------------------------------- |
| `cloudflare` | `cloudflared tunnel` process stdout — `src/tunnel/cloudflare.rs` |
| `ngrok`      | ngrok local API `GET /api/tunnels` — `src/tunnel/ngrok.rs`       |
| `tailscale`  | `tailscale funnel` + hostname — `src/tunnel/tailscale.rs`        |
| `custom`     | user-supplied URL or stdout pattern — `src/tunnel/custom.rs`     |

The tunnel's `start(local_host, local_port)` method returns the public URL string.
This URL is what gets used as the `redirect_uri`.

Example with Cloudflare Tunnel:

```
https://your-subdomain.trycloudflare.com/auth/callback/gmail
```

Google's OAuth2 server sends the browser to this URL after the user grants consent.

---

## 4. Receiving the Authorization Code via the Tunnel

### 4.1 The callback endpoint

The gateway (`src/gateway/mod.rs`) runs as an Axum HTTP server. A new route needs to be
registered to receive the OAuth2 callback:

```
GET /auth/callback/gmail?code=<AUTH_CODE>&state=<STATE>
```

This route handler must:

1. **Validate the `state` parameter** against the CSRF token stored in memory when the
   flow was initiated. Reject mismatches with `400 Bad Request`.
2. **Extract the `code`** query parameter.
3. **Exchange the code** for tokens by calling Google's token endpoint:

```
POST https://oauth2.googleapis.com/token
  grant_type=authorization_code
  code=<AUTH_CODE>
  client_id=<GOOGLE_CLIENT_ID>
  client_secret=<GOOGLE_CLIENT_SECRET>
  redirect_uri=<TUNNEL_PUBLIC_URL>/auth/callback/gmail
  code_verifier=<PKCE_VERIFIER>
```

The exchange pattern mirrors `src/auth/openai_oauth.rs:exchange_code_for_tokens()`,
which posts a form body and parses the JSON response into `TokenSet`.

Google returns:

```json
{
  "access_token": "ya29.a0...",
  "expires_in": 3599,
  "refresh_token": "1//0g...",
  "scope": "https://www.googleapis.com/auth/gmail.send ...",
  "token_type": "Bearer"
}
```

4. The handler stores the `TokenSet` (see §4.2) and returns a success response to the
   browser (an HTML page or redirect to the frontend app confirming the connection).

### 4.2 The in-flight CSRF / PKCE state

Before redirecting the user to Google, the pending PKCE state and CSRF token must be
stored so the callback handler can look them up. The right storage is the skill's dedicated
in-memory or short-lived file-backed state, keyed by the `state` value:

```
pending_oauth_states: HashMap<state_token, (PkceState, initiated_at)>
```

Entries expire after a fixed window (e.g. 10 minutes) to prevent orphaned state
accumulation. This follows the same pattern ZeroClaw already uses for pairing nonces
in `src/security/pairing.rs`.

---

## 5. Token Storage

After a successful token exchange, the tokens are persisted using the `AuthService`
and `AuthProfilesStore` from `src/auth/`.

### 5.1 Storing the token set

```rust
// Pseudocode — mirrors src/auth/mod.rs AuthService::store_openai_tokens()
let token_set = TokenSet {
    access_token:  response.access_token,
    refresh_token: response.refresh_token,
    id_token:      None,
    expires_at:    Some(Utc::now() + Duration::seconds(response.expires_in)),
    token_type:    Some("Bearer".into()),
    scope:         Some(scopes.join(" ")),
};

auth_service
    .store_oauth_tokens("gmail", "default", token_set, None, true)
    .await?;
```

`store_oauth_tokens` calls `AuthProfilesStore::upsert_profile()`, which:

1. Creates or updates an `AuthProfile` with `kind = AuthProfileKind::OAuth`.
2. Serialises the profile data to `~/.zeroclaw/auth-profiles.json` (file-locked via
   `auth-profiles.lock` with a 10-second timeout — `src/auth/profiles.rs:16–17`).

### 5.2 How tokens are encrypted at rest

Before writing to disk, every secret field passes through `SecretStore::encrypt()`
(`src/security/secrets.rs`). The encryption scheme is:

- **Algorithm**: ChaCha20-Poly1305 AEAD (256-bit key)
- **Key file**: `~/.zeroclaw/.secret_key` (permissions 0600, created on first use)
- **Format on disk**: `enc2:<hex(12-byte-nonce ‖ ciphertext ‖ 16-byte-Poly1305-tag)>`
- **Config**: encryption is enabled by default; disable with `secrets.encrypt = false`

The stored profile entry looks like:

```json
{
  "id": "gmail:default",
  "provider": "gmail",
  "profile_name": "default",
  "kind": "oauth",
  "token_set": {
    "access_token": "enc2:a1b2c3...",
    "refresh_token": "enc2:d4e5f6...",
    "expires_at": "2026-03-09T12:00:00Z",
    "token_type": "Bearer",
    "scope": "https://www.googleapis.com/auth/gmail.send ..."
  },
  "created_at": "2026-03-09T11:00:00Z",
  "updated_at": "2026-03-09T11:00:00Z"
}
```

### 5.3 Token refresh

Google access tokens expire after 3600 seconds. Before any Gmail API call, the skill
must check `token_set.is_expiring_within(Duration::from_secs(90))` (the same 90-second
skew used for OpenAI tokens in `src/auth/mod.rs:160`). If expiring:

```
POST https://oauth2.googleapis.com/token
  grant_type=refresh_token
  refresh_token=<stored_refresh_token>
  client_id=<GOOGLE_CLIENT_ID>
  client_secret=<GOOGLE_CLIENT_SECRET>
```

The new `access_token` (and new `refresh_token` if Google rotates it) is written back
to the auth profile via `AuthProfilesStore::update_profile()`.

---

## 6. Initial Email Sync (First 100 Emails)

Once connected, the skill performs a one-time initial sync. This is triggered
immediately after a successful token exchange and runs as a background task.

### 6.1 Fetch message IDs

```
GET https://gmail.googleapis.com/gmail/v1/users/me/messages
  ?maxResults=100
  &labelIds=INBOX
Authorization: Bearer <access_token>
```

Returns up to 100 message descriptors: `[{ "id": "...", "threadId": "..." }, ...]`.

### 6.2 Fetch full message details

For each message ID, fetch the full message:

```
GET https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}
  ?format=full
Authorization: Bearer <access_token>
```

The response contains headers (From, To, Subject, Date), a snippet, and the body parts.

In practice, to avoid 100 sequential requests, use the Gmail batch API:

```
POST https://www.googleapis.com/batch/gmail/v1
Content-Type: multipart/mixed; boundary="batch_boundary"

--batch_boundary
Content-Type: application/http
GET /gmail/v1/users/me/messages/{id1}?format=full
...
```

### 6.3 Storing emails in ZeroClaw memory

Each email is stored via `memory_store` (`src/tools/memory_store.rs`) with:

| Field      | Value                                                             |
| ---------- | ----------------------------------------------------------------- |
| `key`      | `gmail_msg_<message_id>`                                          |
| `content`  | Formatted string: `From: ... Subject: ... Date: ... Snippet: ...` |
| `category` | `MemoryCategory::Custom("gmail")`                                 |

This makes every synced email searchable via `memory_recall` with queries like
`"gmail from:zeroclaw@example.com"`.

For larger body content that exceeds a single memory entry, store the full body as a
separate entry keyed `gmail_body_<message_id>` and the summary/metadata as
`gmail_msg_<message_id>`.

Example stored entries after sync:

```
key:     gmail_msg_18de7f2a1b3c4e5d
content: From: sender@example.com
         To: me@gmail.com
         Subject: Project update
         Date: 2026-03-08T14:32:00Z
         Snippet: "Here is the latest update on the project..."
         Labels: INBOX, UNREAD
category: gmail

key:     gmail_body_18de7f2a1b3c4e5d
content: (full decoded email body text)
category: gmail
```

### 6.4 Sync state tracking

Store the highest-known history ID after sync so incremental sync can pick up from
where it left off:

```
key:     gmail_sync_history_id
content: 12345678
category: core
```

---

## 7. The `gmail_send_email` Tool

### 7.1 Tool definition in `SKILL.toml`

The `[[tools]]` entry from §2:

```toml
[[tools]]
name        = "gmail_send_email"
description = "Send an email via Gmail. Requires a connected Gmail account."
kind        = "http"
command     = "https://gmail.googleapis.com/gmail/v1/users/me/messages/send"
```

### 7.2 How the agent calls it

The LLM emits a tool call:

```json
{
  "name": "gmail_send_email",
  "arguments": {
    "to": "recipient@example.com",
    "subject": "Hello from ZeroClaw",
    "body": "This is a test email sent by the agent."
  }
}
```

### 7.3 Tool dispatch path

```
Agent loop (src/agent/loop_.rs)
  └── dispatches to registered tool by name
        └── HttpRequestTool::execute(args)  [src/tools/http_request.rs]
              ├── Retrieve gmail access_token from AuthService
              │     └── auth_service.get_provider_bearer_token("gmail", None)
              │           └── decrypts enc2: value via SecretStore::decrypt()
              ├── Build RFC 2822 message and base64url-encode it
              ├── POST https://gmail.googleapis.com/gmail/v1/users/me/messages/send
              │     Headers:
              │       Authorization: Bearer <decrypted_access_token>
              │       Content-Type: application/json
              │     Body:
              │       { "raw": "<base64url_encoded_rfc2822_message>" }
              └── Return ToolResult { success, output: "Message sent. ID: <id>" }
```

### 7.4 Building the RFC 2822 message

Gmail's send endpoint requires the email encoded as an RFC 2822 message, then base64url
encoded. The structure:

```
From: me@gmail.com
To: recipient@example.com
Subject: Hello from ZeroClaw
Content-Type: text/plain; charset="UTF-8"
MIME-Version: 1.0

This is a test email sent by the agent.
```

Then the entire string is base64url-encoded (no padding) and placed in the `raw` field:

```json
{ "raw": "RnJvbTogbWVAZ21haWwuY29tCi..." }
```

### 7.5 Security enforcement

Before executing any outbound HTTP call, the `SecurityPolicy` is consulted:

```rust
// src/security/policy.rs
security.enforce_tool_operation(ToolOperation::Act, "gmail_send_email")
```

`ToolOperation::Act` is the category for write/side-effect operations. If the agent is
running in `ReadOnly` or `Supervised` mode, this call fails with an explicit error before
any network request is made.

The `HttpRequestTool` additionally enforces the `http.allowed_domains` allowlist. For
Gmail tools, `googleapis.com` must be present in that list:

```toml
# config.toml
[http]
enabled         = true
allowed_domains = ["googleapis.com"]
```

### 7.6 Reply-to and threading

To send a reply within an existing thread, add the Gmail `threadId` to the request body:

```json
{ "raw": "<base64url_message_with_In-Reply-To_header>", "threadId": "18de7f2a1b3c4e5d" }
```

The RFC 2822 message must include `In-Reply-To: <original_message_id>` and
`References: <original_message_id>` headers for proper threading.

---

## 8. Full End-to-End Sequence

```
Frontend / User
    │
    │  1. "Connect Gmail"
    ▼
ZeroClaw Backend
    │  2. Generate PKCE + CSRF state
    │  3. Build Google OAuth2 authorize URL with tunnel redirect_uri
    │  4. Return URL to frontend
    │
    ▼
Browser (user)
    │  5. User visits URL, grants consent in Google
    │
    ▼
Google OAuth2
    │  6. Browser redirected to <TUNNEL_PUBLIC_URL>/auth/callback/gmail?code=XXX&state=YYY
    │
    ▼
Tunnel (cloudflare / ngrok / tailscale)
    │  7. Proxies HTTPS request to local gateway port
    │
    ▼
Gateway (src/gateway/mod.rs) — GET /auth/callback/gmail
    │  8. Validate CSRF state
    │  9. Exchange code → TokenSet via POST https://oauth2.googleapis.com/token
    │ 10. Store TokenSet in AuthProfilesStore (encrypted, "gmail:default")
    │ 11. Return success page/redirect to frontend
    │
    ▼
Background task — initial sync
    │ 12. GET /gmail/v1/users/me/messages?maxResults=100&labelIds=INBOX
    │ 13. Batch-fetch 100 full messages
    │ 14. memory_store each email under key=gmail_msg_<id>, category=gmail
    │ 15. memory_store gmail_sync_history_id = <latest_historyId>
    │
    ▼
Agent (subsequent interactions)
    │ 16. LLM generates tool call: gmail_send_email { to, subject, body }
    │ 17. Agent loop dispatches to HttpRequestTool
    │ 18. HttpRequestTool retrieves + decrypts access_token from AuthService
    │ 19. Refreshes token if expiring within 90 seconds
    │ 20. POST https://gmail.googleapis.com/gmail/v1/users/me/messages/send
    │ 21. Returns ToolResult to agent
    ▼
Done
```

---

## 9. Configuration Reference

All Gmail skill config lives in `config.toml`:

```toml
# Enable HTTP request tool with Google API access
[http]
enabled         = true
allowed_domains = ["googleapis.com", "oauth2.googleapis.com"]
timeout_secs    = 30
max_response_size = 524288  # 512KB — enough for email batch responses

# Tunnel for OAuth callback (pick one)
[tunnel]
provider = "cloudflare"
[tunnel.cloudflare]
token = "your-cloudflare-tunnel-token"

# Or ngrok:
# [tunnel]
# provider = "ngrok"
# [tunnel.ngrok]
# auth_token = "your-ngrok-token"

# Google OAuth2 app credentials (stored encrypted)
# Set via environment or onboard wizard — never commit raw values
[integrations.gmail]
client_id     = "enc2:..."
client_secret = "enc2:..."
```

The Google credentials (`client_id`, `client_secret`) are encrypted by `SecretStore`
before being written to `config.toml` — the same ChaCha20-Poly1305 scheme used for all
secrets (`src/security/secrets.rs`).

---

## 10. Key Source Files

| File                         | Role                                                              |
| ---------------------------- | ----------------------------------------------------------------- |
| `src/skills/mod.rs`          | Skill loading, `SkillTool` struct, `load_skills_with_config()`    |
| `src/auth/mod.rs`            | `AuthService` — store/retrieve/refresh OAuth tokens               |
| `src/auth/profiles.rs`       | `AuthProfile`, `TokenSet`, `AuthProfilesStore` — JSON persistence |
| `src/auth/openai_oauth.rs`   | PKCE generation, code exchange, refresh — reference pattern       |
| `src/security/secrets.rs`    | `SecretStore` — ChaCha20-Poly1305 encrypt/decrypt                 |
| `src/tunnel/mod.rs`          | `Tunnel` trait + factory — public URL for OAuth redirect          |
| `src/tunnel/cloudflare.rs`   | Cloudflare Tunnel implementation                                  |
| `src/tunnel/ngrok.rs`        | ngrok implementation                                              |
| `src/gateway/mod.rs`         | Axum HTTP gateway — where `/auth/callback/gmail` is registered    |
| `src/tools/http_request.rs`  | `HttpRequestTool` — dispatches Gmail API calls                    |
| `src/tools/memory_store.rs`  | `MemoryStoreTool` — stores synced emails                          |
| `src/tools/memory_recall.rs` | `MemoryRecallTool` — searches synced emails                       |
| `src/security/policy.rs`     | `SecurityPolicy` — enforces `ToolOperation::Act` guards           |
| `docs/config-reference.md`   | Full config schema including `[http]`, `[tunnel]`                 |

---

## 11. Security Notes

- The OAuth2 `redirect_uri` **must be the tunnel URL**. It cannot be `localhost` in a
  remote/mobile scenario. Google validates the exact URI registered in the Google Cloud
  Console; register it as `https://<your-tunnel-domain>/auth/callback/gmail`.
- The CSRF `state` token must be validated on every callback. Reject mismatched or
  missing state with `400 Bad Request` before touching any tokens.
- The PKCE verifier must be destroyed after a successful or failed exchange — never
  persist it beyond the in-flight flow.
- `client_secret` must never appear in plaintext in `config.toml`, logs, or agent
  tool output. Encrypt it via the secret store and redact it in observability output.
- `googleapis.com` must be explicitly present in `http.allowed_domains` — the
  `HttpRequestTool` enforces this allowlist before every request
  (`src/tools/http_request.rs`).
- Token refresh runs under the same per-profile Tokio mutex used for OpenAI tokens
  (`src/auth/mod.rs:287`) to prevent double-refresh races.
