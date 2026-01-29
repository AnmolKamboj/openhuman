/**
 * Telegram MCP Server
 * Main entry point for Telegram MCP integration
 */

import type { Socket } from "socket.io-client";
import { TelegramMCPServer } from "./server";

let telegramMCPInstance: TelegramMCPServer | undefined;

export function initTelegramMCPServer(
  socket: Socket | null | undefined,
): TelegramMCPServer {
  telegramMCPInstance = new TelegramMCPServer(socket);
  console.log("[MCP] Telegram MCP server initialized");
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
    console.log("[MCP] Telegram MCP server socket updated");
  }
}

export function cleanupTelegramMCPServer(): void {
  if (telegramMCPInstance) {
    telegramMCPInstance = undefined;
    console.log("[MCP] Telegram MCP server cleaned up");
  }
}

export { toHumanReadableAction } from "./toolActionParser";
export type { TelegramMCPServer } from "./server";
