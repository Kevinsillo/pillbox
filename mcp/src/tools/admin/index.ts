/**
 * Tools de administración: stats y bottle_create.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../../exec.js";
import { fromExecResult } from "../../response.js";
import { BottleCreateSchema } from "../../schemas.js";

export function registerAdminTools(server: McpServer): void {
  // ── bottle_create ───────────────────────────────────────────────────────────
  server.tool(
    "bottle_create",
    "Registra un nuevo bottle (proyecto) en Pillbox. " +
      "Normalmente lo hace `pillbox bottle init`; esta tool existe para automatización.",
    BottleCreateSchema.shape,
    async (input) => {
      const result = pillboxExec("bottle_create", input);
      return fromExecResult(result);
    },
  );

  // ── stats ───────────────────────────────────────────────────────────────────
  server.tool(
    "stats",
    "Devuelve el listado de bottles con conteo de prescripciones y pills activas.",
    {},
    async () => {
      // Reutiliza bottle_list — el store ya devuelve los datos básicos
      const result = pillboxExec("bottle_list", {});
      return fromExecResult(result);
    },
  );
}
