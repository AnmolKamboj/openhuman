#!/usr/bin/env python3
"""
Hello Python — Example Runtime Skill

Demonstrates the JSON-RPC 2.0 protocol for AlphaHuman runtime skills.
The host communicates over stdin/stdout with newline-delimited JSON.

Protocol:
  Host → Skill:  JSON-RPC requests on stdin (one JSON object per line)
  Skill → Host:  JSON-RPC responses on stdout (one JSON object per line)
  Skill logging:  stderr (forwarded to host debug log)

Lifecycle:
  1. Host spawns this process
  2. Host sends  skill/load   { manifest }
  3. Host sends  tools/list   (skill replies with tool definitions)
  4. Host sends  skill/activate
  5. Host sends  tools/call   as needed
  6. Host sends  skill/deactivate  then  skill/shutdown
"""

import json
import sys


def send(obj: dict) -> None:
    """Write a JSON-RPC message to stdout."""
    line = json.dumps(obj, separators=(",", ":"))
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


def send_result(req_id, result):
    """Send a successful JSON-RPC response."""
    send({"jsonrpc": "2.0", "id": req_id, "result": result})


def send_error(req_id, code: int, message: str):
    """Send a JSON-RPC error response."""
    send({"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}})


def log(message: str) -> None:
    """Log to stderr (host captures this for debugging)."""
    print(f"[hello-python] {message}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------------------
# Tool definitions
# ---------------------------------------------------------------------------

TOOLS = [
    {
        "name": "hello_world",
        "description": "Returns a friendly greeting. Use this to verify the Python runtime skill is working.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name to greet (default: 'World')",
                }
            },
        },
        "tier": "state_only",
        "readOnly": True,
    }
]


def handle_tool_call(name: str, arguments: dict) -> dict:
    """Execute a tool and return an MCPToolResult."""
    if name == "hello_world":
        who = arguments.get("name", "World")
        return {
            "content": [{"type": "text", "text": f"Hello, {who}! Greetings from a Python runtime skill."}],
        }

    return {
        "content": [{"type": "text", "text": f"Unknown tool: {name}"}],
        "isError": True,
    }


# ---------------------------------------------------------------------------
# Request dispatcher
# ---------------------------------------------------------------------------

def handle_request(method: str, params, req_id):
    """Dispatch a JSON-RPC request to the appropriate handler."""

    if method == "skill/load":
        manifest = params.get("manifest", {}) if params else {}
        log(f"Loaded — manifest id: {manifest.get('id', '?')}")
        send_result(req_id, {"ok": True})

    elif method == "skill/activate":
        log("Activated")
        send_result(req_id, {"ok": True})

    elif method == "skill/deactivate":
        log("Deactivated")
        send_result(req_id, {"ok": True})

    elif method == "skill/unload":
        log("Unloaded")
        send_result(req_id, {"ok": True})

    elif method == "skill/shutdown":
        log("Shutting down")
        send_result(req_id, {"ok": True})
        sys.exit(0)

    elif method == "skill/sessionStart":
        session_id = params.get("sessionId", "?") if params else "?"
        log(f"Session started: {session_id}")
        send_result(req_id, {"ok": True})

    elif method == "skill/sessionEnd":
        session_id = params.get("sessionId", "?") if params else "?"
        log(f"Session ended: {session_id}")
        send_result(req_id, {"ok": True})

    elif method == "skill/beforeMessage":
        # Return None to leave message unchanged, or return transformed message
        send_result(req_id, {})

    elif method == "skill/afterResponse":
        # Return None to leave response unchanged
        send_result(req_id, {})

    elif method == "skill/tick":
        # Health check / periodic tick
        send_result(req_id, {"ok": True})

    elif method == "tools/list":
        send_result(req_id, {"tools": TOOLS})

    elif method == "tools/call":
        name = params.get("name", "") if params else ""
        arguments = params.get("arguments", {}) if params else {}
        result = handle_tool_call(name, arguments)
        send_result(req_id, result)

    else:
        send_error(req_id, -32601, f"Method not found: {method}")


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------

def main():
    log("Starting hello-python skill")

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except json.JSONDecodeError as e:
            log(f"Invalid JSON: {e}")
            continue

        # Must be JSON-RPC 2.0
        if msg.get("jsonrpc") != "2.0":
            log(f"Not a JSON-RPC 2.0 message: {line[:100]}")
            continue

        method = msg.get("method")
        params = msg.get("params")
        req_id = msg.get("id")

        if method and req_id is not None:
            # Request — needs a response
            handle_request(method, params, req_id)
        elif method:
            # Notification — no response needed
            log(f"Notification: {method}")
        else:
            log(f"Unrecognized message: {line[:100]}")


if __name__ == "__main__":
    main()
