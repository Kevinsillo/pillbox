/**
 * Tools de administración: stats y bottle_create.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../../exec.js";
import { fromExecResult } from "../../response.js";
import { BottleCreateSchema } from "../../schemas.js";

export function registerAdminTools(server: McpServer): void {
  server.registerTool(
    "bottle_create",
    {
      description:
        "Registra un nuevo bottle (proyecto) en Pillbox. " +
        "Normalmente lo hace `pillbox bottle init`; esta tool existe para automatización.",
      inputSchema: BottleCreateSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("bottle_create", input)),
  );

  server.registerTool(
    "stats",
    {
      description: "Devuelve el listado de bottles con sus datos básicos.",
      inputSchema: {},
    },
    async () => fromExecResult(pillboxExec("bottle_list", {})),
  );

  server.registerTool(
    "pill_compounds",
    {
      description:
        "Devuelve los compounds disponibles para pill_take, con descripción y prompt_hint. " +
        "Consultar antes de pill_take si no estás seguro de qué compound usar.",
      inputSchema: {},
    },
    async () => fromExecResult(pillboxExec("pill_compounds", {})),
  );

  server.registerTool(
    "capsule_compounds",
    {
      description:
        "Devuelve los compounds disponibles para capsule_take, con descripción y prompt_hint. " +
        "Consultar antes de capsule_take si no estás seguro de qué compound usar.",
      inputSchema: {},
    },
    async () => fromExecResult(pillboxExec("capsule_compounds", {})),
  );
}
