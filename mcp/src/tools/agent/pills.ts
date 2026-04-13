/**
 * Tools de pills — conocimiento específico de proyecto.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../../exec.js";
import { fromExecResult } from "../../response.js";
import {
  PillTakeSchema,
  PillReadSchema,
  PillReviseSchema,
  PillDiscardSchema,
  PillFindSchema,
  PillContextSchema,
} from "../../schemas.js";

export function registerPillTools(server: McpServer): void {
  // ── pill_take ───────────────────────────────────────────────────────────────
  server.tool(
    "pill_take",
    "Guarda una nueva pill (conocimiento de proyecto) en una prescripción abierta. " +
      "Compounds disponibles: decision, context, problem, solution, learning, " +
      "reference, task, prescription_summary.",
    PillTakeSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_take", input);
      return fromExecResult(result);
    },
  );

  // ── pill_read ───────────────────────────────────────────────────────────────
  server.tool(
    "pill_read",
    "Lee el contenido completo de una pill por su ID.",
    PillReadSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_read", input);
      return fromExecResult(result);
    },
  );

  // ── pill_revise ─────────────────────────────────────────────────────────────
  server.tool(
    "pill_revise",
    "Actualiza el título y/o contenido de una pill existente. " +
      "Solo los campos presentes en `patch` se modifican.",
    PillReviseSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_revise", input);
      return fromExecResult(result);
    },
  );

  // ── pill_discard ────────────────────────────────────────────────────────────
  server.tool(
    "pill_discard",
    "Hace soft-delete de una pill. No se puede deshacer.",
    PillDiscardSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_discard", input);
      return fromExecResult(result);
    },
  );

  // ── pill_find ───────────────────────────────────────────────────────────────
  server.tool(
    "pill_find",
    "Busca pills usando búsqueda full-text (FTS5). " +
      "Acepta múltiples términos separados por espacios. " +
      "Filtra opcionalmente por bottle_id o compound.",
    PillFindSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_find", input);
      return fromExecResult(result);
    },
  );

  // ── pill_context ────────────────────────────────────────────────────────────
  server.tool(
    "pill_context",
    "Obtiene el contexto reciente de un bottle en formato Markdown: " +
      "prescripciones recientes y sus pills. " +
      "Usar al inicio de una sesión para recuperar el estado del proyecto.",
    PillContextSchema.shape,
    async (input) => {
      const result = pillboxExec("pill_context", input);
      return fromExecResult(result);
    },
  );
}
