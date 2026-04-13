/**
 * Tools de gestión de prescripciones (sesiones de trabajo).
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../exec.js";
import { fromExecResult, mcpOk } from "../response.js";
import {
  PrescriptionOpenSchema,
  PrescriptionCloseSchema,
  PrescriptionReadSchema,
  PrescriptionDiscardSchema,
} from "../schemas.js";

export function registerPrescriptionTools(server: McpServer): void {
  // ── prescription_open ───────────────────────────────────────────────────────
  server.tool(
    "prescription_open",
    "Abre una nueva prescripción (sesión de trabajo) para un bottle. " +
      "Devuelve el ID de la prescripción creada. " +
      "Si ya hay una abierta, devuelve error `prescription_already_open` con sus datos.",
    PrescriptionOpenSchema.shape,
    async (input) => {
      const result = pillboxExec("prescription_open", input);
      // Error tipado: prescription_already_open incluye campo data con la existente
      return fromExecResult(result);
    },
  );

  // ── prescription_close ──────────────────────────────────────────────────────
  server.tool(
    "prescription_close",
    "Cierra una prescripción abierta. Finaliza la sesión de trabajo.",
    PrescriptionCloseSchema.shape,
    async (input) => {
      const result = pillboxExec("prescription_close", input);
      return fromExecResult(result);
    },
  );

  // ── prescription_read ───────────────────────────────────────────────────────
  server.tool(
    "prescription_read",
    "Lee los detalles de una prescripción por su ID.",
    PrescriptionReadSchema.shape,
    async (input) => {
      const result = pillboxExec("prescription_read", input);
      return fromExecResult(result);
    },
  );

  // ── prescription_discard ────────────────────────────────────────────────────
  server.tool(
    "prescription_discard",
    "Descarta una prescripción y hace soft-delete en cascada de todas sus pills.",
    PrescriptionDiscardSchema.shape,
    async (input) => {
      const result = pillboxExec("prescription_discard", input);
      return fromExecResult(result);
    },
  );

  // ── bottle_list ─────────────────────────────────────────────────────────────
  server.tool(
    "bottle_list",
    "Lista todos los bottles (proyectos) registrados en Pillbox.",
    {},
    async () => {
      const result = pillboxExec("bottle_list", {});
      return fromExecResult(result);
    },
  );
}
