/**
 * API layer types for Telegram MCP
 *
 * Every API function returns ApiResult<T> which carries both the data
 * and a `fromCache` boolean indicating whether it was served from Redux
 * cache or fetched from the Telegram API.
 */

export interface ApiResult<T> {
  data: T;
  fromCache: boolean;
}
