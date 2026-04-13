/**
 * Helpers para construir respuestas MCP.
 *
 * El SDK espera { content: [{ type: "text", text: string }], isError?: boolean }
 */

import type { ExecResult } from "./exec.js";

export interface McpContent {
  type: "text";
  text: string;
}

export interface McpResponse {
  [key: string]: unknown;
  content: McpContent[];
  isError?: boolean;
}

export function mcpOk(data: unknown): McpResponse {
  return {
    content: [{ type: "text", text: JSON.stringify(data, null, 2) }],
  };
}

export function mcpErr(error: string, message: string, data?: unknown): McpResponse {
  const body: Record<string, unknown> = { error, message };
  if (data !== undefined) body.data = data;
  return {
    content: [{ type: "text", text: JSON.stringify(body, null, 2) }],
    isError: true,
  };
}

export function fromExecResult(result: ExecResult): McpResponse {
  if (result.ok) {
    return mcpOk(result.data);
  }
  return mcpErr(result.error, result.message, result.data);
}
