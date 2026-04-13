/**
 * Tools de capsules — conocimiento personal cross-proyecto.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../../exec.js";
import { fromExecResult } from "../../response.js";
import {
  CapsuleTakeSchema,
  CapsuleReadSchema,
  CapsuleReviseSchema,
  CapsuleDiscardSchema,
  CapsuleFindSchema,
} from "../../schemas.js";

export function registerCapsuleTools(server: McpServer): void {
  // ── capsule_take ────────────────────────────────────────────────────────────
  server.tool(
    "capsule_take",
    "Guarda una nueva capsule (conocimiento personal del usuario, cross-proyecto). " +
      "Compounds disponibles: convention, workflow, environment, context, goal, manual.",
    CapsuleTakeSchema.shape,
    async (input) => {
      const result = pillboxExec("capsule_take", input);
      return fromExecResult(result);
    },
  );

  // ── capsule_read ────────────────────────────────────────────────────────────
  server.tool(
    "capsule_read",
    "Lee el contenido completo de una capsule por su ID.",
    CapsuleReadSchema.shape,
    async (input) => {
      const result = pillboxExec("capsule_read", input);
      return fromExecResult(result);
    },
  );

  // ── capsule_revise ──────────────────────────────────────────────────────────
  server.tool(
    "capsule_revise",
    "Actualiza el título y/o contenido de una capsule existente.",
    CapsuleReviseSchema.shape,
    async (input) => {
      const result = pillboxExec("capsule_revise", input);
      return fromExecResult(result);
    },
  );

  // ── capsule_discard ─────────────────────────────────────────────────────────
  server.tool(
    "capsule_discard",
    "Hace soft-delete de una capsule.",
    CapsuleDiscardSchema.shape,
    async (input) => {
      const result = pillboxExec("capsule_discard", input);
      return fromExecResult(result);
    },
  );

  // ── capsule_find ────────────────────────────────────────────────────────────
  server.tool(
    "capsule_find",
    "Busca capsules usando búsqueda full-text (FTS5). " +
      "Las capsules son globales — no se filtran por proyecto.",
    CapsuleFindSchema.shape,
    async (input) => {
      const result = pillboxExec("capsule_find", input);
      return fromExecResult(result);
    },
  );
}
