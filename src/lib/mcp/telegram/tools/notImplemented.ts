import type { MCPToolResult } from '../../types';

export function notImplemented(name: string): MCPToolResult {
  return {
    content: [{ type: 'text', text: `${name} is not implemented yet.` }],
    isError: true,
  };
}
