/**
 * JSON-RPC 2.0 transport over Tauri shell plugin (stdin/stdout).
 *
 * Spawns a subprocess via Command.create(), sends JSON-RPC requests
 * via stdin, and reads newline-delimited JSON responses from stdout.
 * Also handles reverse RPC from the skill process (state/get, state/set,
 * data/read, data/write).
 */

import { Command, type Child } from "@tauri-apps/plugin-shell";
import type { JsonRpcRequest } from "./types";

const REQUEST_TIMEOUT_MS = 30_000;

/** Bundled runtimes (must match bundle.externalBin and capability allow[].name) */
const SIDECAR_RUNTIMES: Record<string, string> = {
  "runtime-skill-python": "runtime-skill-python",
};

function getSidecarName(program: string): string | null {
  return SIDECAR_RUNTIMES[program] ?? null;
}

export type ReverseRpcHandler = (
  method: string,
  params: Record<string, unknown>
) => Promise<unknown>;

export class SkillTransport {
  private child: Child | null = null;
  private command: Command<string> | null = null;
  private nextId = 1;
  private pending = new Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  >();
  private reverseRpcHandler: ReverseRpcHandler | null = null;
  private _killed = false;

  /**
   * Set a handler for reverse RPC calls from the skill process.
   * The handler receives (method, params) and returns the result.
   */
  onReverseRpc(handler: ReverseRpcHandler): void {
    this.reverseRpcHandler = handler;
  }

  /**
   * Spawn the skill process. Uses sidecar when program is a bundled runtime
   * (e.g. runtime-skill-python); otherwise direct program execution (program + args).
   * Program name must match capability allow[].name.
   */
  async start(
    program: string,
    args: string[],
    env?: Record<string, string>,
    cwd?: string
  ): Promise<void> {
    this._killed = false;

    const opts: Record<string, unknown> = {};
    if (env) opts.env = env;
    if (cwd) opts.cwd = cwd;

    const sidecarName = getSidecarName(program);

    // For sidecar, always pass cwd and env explicitly
    const spawnOpts =
      sidecarName && (env || cwd)
        ? {
            ...(cwd && { cwd }),
            ...(env && { env }),
          }
        : undefined;

    // Log spawn details for debugging
    console.log("[skill-transport] Spawning:", {
      program,
      sidecarName,
      args,
      cwd,
      env: env
        ? Object.keys(env).map((k) => `${k}=${env[k]?.substring(0, 100)}`)
        : undefined,
      usingSidecar: !!sidecarName,
      spawnOptsKeys: spawnOpts ? Object.keys(spawnOpts) : undefined,
    });

    this.command = sidecarName
      ? Command.sidecar(sidecarName, args, spawnOpts)
      : Command.create(program, args, opts);

    // Handle stdout — newline-delimited JSON-RPC messages
    this.command.stdout.on("data", (line: string) => {
      const trimmed = line.trim();
      if (!trimmed) return;
      try {
        const msg = JSON.parse(trimmed);
        console.debug("[skill-transport] Received stdout message", {
          hasId: "id" in msg,
          hasMethod: "method" in msg,
        });
        this.handleMessage(msg);
      } catch {
        console.debug("[skill-transport] Non-JSON stdout:", trimmed);
      }
    });

    // Handle stderr — skill debug logs and errors
    this.command.stderr.on("data", (line: string) => {
      const trimmed = line.trimEnd();
      if (trimmed) {
        // Log Python errors prominently
        console.error("[skill-stderr]", trimmed);
      }
    });

    this.command.on(
      "close",
      (data: { code: number | null; signal: number | null }) => {
        if (!this._killed) {
          const exitCode = data.code ?? "unknown";
          const signal = data.signal ?? "none";
          console.error(
            `[skill-transport] Process exited with code: ${exitCode}, signal: ${signal}`
          );
        }
        this.rejectAll(
          new Error(`Skill process exited with code: ${data.code ?? "unknown"}`)
        );
      }
    );

    this.command.on("error", (error: string) => {
      console.error("[skill-transport] Process error:", error);
      this.rejectAll(new Error(`Skill process error: ${error}`));
    });

    this.child = await this.command.spawn();
  }

  /**
   * Send a JSON-RPC request and return a promise for the result.
   */
  async request<T = unknown>(
    method: string,
    params?: Record<string, unknown>
  ): Promise<T> {
    if (!this.child) {
      throw new Error("Skill process not started");
    }

    const id = this.nextId++;
    const msg: Record<string, unknown> = {
      jsonrpc: "2.0",
      id,
      method,
    };
    if (params !== undefined) {
      msg.params = params;
    }

    console.log("[skill-transport] Sending request", {
      id,
      method,
      hasParams: params !== undefined,
    });

    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        console.error("[skill-transport] Request timeout", {
          id,
          method,
          timeoutMs: REQUEST_TIMEOUT_MS,
        });
        reject(new Error(`JSON-RPC timeout: ${method}`));
      }, REQUEST_TIMEOUT_MS);

      this.pending.set(id, {
        resolve: resolve as (v: unknown) => void,
        reject,
        timer,
      });

      this.writeMessage(msg);
    });
  }

  /**
   * Send a JSON-RPC notification (no response expected).
   */
  notify(method: string, params?: Record<string, unknown>): void {
    if (!this.child) {
      console.warn("[skill-transport] Cannot notify - process not started", {
        method,
      });
      return;
    }

    const msg: Record<string, unknown> = {
      jsonrpc: "2.0",
      method,
    };
    if (params !== undefined) {
      msg.params = params;
    }

    console.log("[skill-transport] Sending notification", {
      method,
      hasParams: params !== undefined,
    });
    this.writeMessage(msg);
  }

  /**
   * Terminate the skill process.
   */
  async kill(): Promise<void> {
    this._killed = true;
    this.rejectAll(new Error("Skill process killed"));
    if (this.child) {
      try {
        await this.child.kill();
      } catch {
        // Process may already be dead
      }
      this.child = null;
    }
    this.command = null;
  }

  get isRunning(): boolean {
    return this.child !== null && !this._killed;
  }

  // -----------------------------------------------------------------------
  // Internal
  // -----------------------------------------------------------------------

  private handleMessage(msg: Record<string, unknown>): void {
    // Response to our request
    if ("id" in msg && ("result" in msg || "error" in msg)) {
      const id = msg.id as number;
      const hasError = "error" in msg;
      console.log("[skill-transport] Received response", {
        id,
        hasError,
        hasResult: !hasError,
      });
      const pending = this.pending.get(id);
      if (pending) {
        this.pending.delete(id);
        clearTimeout(pending.timer);
        if ("error" in msg) {
          const err = msg.error as { message?: string };
          console.error("[skill-transport] Response error", {
            id,
            error: err?.message,
          });
          pending.reject(new Error(err?.message ?? "JSON-RPC error"));
        } else {
          pending.resolve(msg.result);
        }
      } else {
        console.warn(
          "[skill-transport] No pending request found for response",
          { id }
        );
      }
      return;
    }

    // Reverse RPC request from skill
    if ("method" in msg && "id" in msg) {
      const rpcMsg = msg as unknown as JsonRpcRequest;
      console.log("[skill-transport] Received reverse RPC request", {
        id: rpcMsg.id,
        method: rpcMsg.method,
      });
      this.handleReverseRpc(rpcMsg);
      return;
    }
  }

  private async handleReverseRpc(msg: JsonRpcRequest): Promise<void> {
    if (!this.reverseRpcHandler) {
      console.error("[skill-transport] No reverse RPC handler registered", {
        method: msg.method,
        id: msg.id,
      });
      this.sendResponse(msg.id, undefined, {
        code: -32601,
        message: "No reverse RPC handler registered",
      });
      return;
    }

    try {
      console.log("[skill-transport] Handling reverse RPC", {
        method: msg.method,
        id: msg.id,
      });
      const result = await this.reverseRpcHandler(
        msg.method,
        (msg.params ?? {}) as Record<string, unknown>
      );
      console.log("[skill-transport] Reverse RPC success", {
        method: msg.method,
        id: msg.id,
      });
      this.sendResponse(msg.id, result);
    } catch (err: unknown) {
      console.error("[skill-transport] Reverse RPC error", {
        method: msg.method,
        id: msg.id,
        error: err instanceof Error ? err.message : String(err),
      });
      this.sendResponse(msg.id, undefined, {
        code: -32603,
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

  private sendResponse(
    id: number,
    result?: unknown,
    error?: { code: number; message: string }
  ): void {
    const response: Record<string, unknown> = { jsonrpc: "2.0", id };
    if (error) {
      response.error = error;
    } else {
      response.result = result ?? null;
    }
    this.writeMessage(response);
  }

  private writeMessage(msg: Record<string, unknown>): void {
    if (!this.child) {
      console.warn(
        "[skill-transport] Cannot write message - process not started",
        { method: msg.method }
      );
      return;
    }
    const data = JSON.stringify(msg) + "\n";
    console.debug("[skill-transport] Writing message", {
      method: msg.method,
      id: msg.id,
      dataSize: data.length,
    });
    this.child.write(data).catch((err: unknown) => {
      console.error("[skill-transport] Write error:", err);
    });
  }

  private rejectAll(error: Error): void {
    for (const [id, pending] of this.pending) {
      clearTimeout(pending.timer);
      pending.reject(error);
      this.pending.delete(id);
    }
  }
}
