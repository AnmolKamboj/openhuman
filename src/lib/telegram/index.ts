/**
 * Telegram MCP Server
 * Main entry point for Telegram MCP integration
 */

import type { Socket } from "socket.io-client";
import createDebug from "debug";
import { TelegramMCPServer } from "./server";

const log = createDebug("app:telegram:mcp");

let telegramMCPInstance: TelegramMCPServer | undefined;

export function initTelegramMCPServer(
  socket: Socket | null | undefined,
): TelegramMCPServer {
  telegramMCPInstance = new TelegramMCPServer(socket);
  log("Telegram MCP server initialized");
  return telegramMCPInstance;
}

export function getTelegramMCPServer(): TelegramMCPServer | undefined {
  return telegramMCPInstance;
}

export function updateTelegramMCPServerSocket(
  socket: Socket | null | undefined,
): void {
  if (telegramMCPInstance) {
    telegramMCPInstance.updateSocket(socket);
    log("Telegram MCP server socket updated");
  }
}

export function cleanupTelegramMCPServer(): void {
  if (telegramMCPInstance) {
    telegramMCPInstance = undefined;
    log("Telegram MCP server cleaned up");
  }
}

export { toHumanReadableAction } from "./toolActionParser";
export type { TelegramMCPServer } from "./server";
