/**
 * Pillbox MCP Server.
 *
 * Expone las tools de Pillbox al agente vía el Model Context Protocol.
 * Comunica con el binario `pillbox` mediante `pillbox exec` (stdin/stdout JSON).
 *
 * Uso en Claude Code (claude_desktop_config.json):
 *   {
 *     "mcpServers": {
 *       "pillbox": {
 *         "command": "node",
 *         "args": ["/path/to/pillbox-mcp/dist/index.js"]
 *       }
 *     }
 *   }
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

import { registerPrescriptionTools } from "./tools/prescription.js";
import { registerPillTools } from "./tools/agent/pills.js";
import { registerCapsuleTools } from "./tools/agent/capsules.js";
import { registerAdminTools } from "./tools/admin/index.js";

const server = new McpServer({
  name: "pillbox",
  version: "0.1.0",
});

registerPrescriptionTools(server);
registerPillTools(server);
registerCapsuleTools(server);
registerAdminTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
