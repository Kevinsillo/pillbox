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
  server.registerTool(
    "pill_take",
    {
      description:
        "Guarda conocimiento específico del proyecto en una prescripción abierta. " +
        "Usar en lugar de capsule_take cuando el conocimiento pertenece a este proyecto concreto — " +
        "requiere prescription_id activa. " +
        "Ejemplos: decisiones de arquitectura, bugs corregidos, patrones del código. " +
        "Para elegir compound, llamar pill_compounds.",
      inputSchema: PillTakeSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_take", input)),
  );

  server.registerTool(
    "pill_read",
    {
      description: "Lee el contenido completo de una pill por su ID.",
      inputSchema: PillReadSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_read", input)),
  );

  server.registerTool(
    "pill_revise",
    {
      description:
        "Actualiza el título y/o contenido de una pill existente. " +
        "Solo los campos presentes en `patch` se modifican.",
      inputSchema: PillReviseSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_revise", input)),
  );

  server.registerTool(
    "pill_discard",
    {
      description: "Hace soft-delete de una pill. No se puede deshacer.",
      inputSchema: PillDiscardSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_discard", input)),
  );

  server.registerTool(
    "pill_search",
    {
      description:
        "Busca pills usando búsqueda full-text (FTS5). " +
        "Acepta múltiples términos separados por espacios. " +
        "Filtra opcionalmente por bottle_id o compound.",
      inputSchema: PillFindSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_search", input)),
  );

  server.registerTool(
    "pill_context",
    {
      description:
        "Obtiene el contexto reciente de un bottle en formato Markdown: " +
        "prescripciones recientes y sus pills. " +
        "Usar al inicio de una sesión para recuperar el estado del proyecto.",
      inputSchema: PillContextSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("pill_context", input)),
  );
}
