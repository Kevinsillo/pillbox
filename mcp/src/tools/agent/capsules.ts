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
  server.registerTool(
    "capsule_take",
    {
      description:
        "Guarda una nueva capsule (conocimiento personal del usuario, cross-proyecto). " +
        "Compounds disponibles: convention, workflow, environment, context, goal, feedback, manual.",
      inputSchema: CapsuleTakeSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("capsule_take", input)),
  );

  server.registerTool(
    "capsule_read",
    {
      description: "Lee el contenido completo de una capsule por su ID.",
      inputSchema: CapsuleReadSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("capsule_read", input)),
  );

  server.registerTool(
    "capsule_revise",
    {
      description: "Actualiza el título y/o contenido de una capsule existente.",
      inputSchema: CapsuleReviseSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("capsule_revise", input)),
  );

  server.registerTool(
    "capsule_discard",
    {
      description: "Hace soft-delete de una capsule.",
      inputSchema: CapsuleDiscardSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("capsule_discard", input)),
  );

  server.registerTool(
    "capsule_find",
    {
      description:
        "Busca capsules usando búsqueda full-text (FTS5). " +
        "Las capsules son globales — no se filtran por proyecto.",
      inputSchema: CapsuleFindSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("capsule_find", input)),
  );
}
