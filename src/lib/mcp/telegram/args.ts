/**
 * Helpers for reading typed values from MCP tool args (Record<string, unknown>)
 */

export function optNumber(
  args: Record<string, unknown>,
  key: string,
  fallback: number,
): number {
  const v = args[key];
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}

export function optString(
  args: Record<string, unknown>,
  key: string,
): string | undefined {
  const v = args[key];
  return typeof v === "string" ? v : undefined;
}
