/**
 * Tools de gestión de prescripciones (sesiones de trabajo).
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { pillboxExec } from "../exec.js";
import { fromExecResult } from "../response.js";
import {
  PrescriptionOpenSchema,
  PrescriptionCloseSchema,
  PrescriptionReadSchema,
  PrescriptionDiscardSchema,
} from "../schemas.js";

export function registerPrescriptionTools(server: McpServer): void {
  server.registerTool(
    "prescription_open",
    {
      description:
        "Abre una nueva prescripción (sesión de trabajo) para un bottle. " +
        "Devuelve el ID de la prescripción creada. " +
        "Si ya hay una abierta, devuelve error `prescription_already_open` con sus datos.",
      inputSchema: PrescriptionOpenSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("prescription_open", input)),
  );

  server.registerTool(
    "prescription_close",
    {
      description: "Cierra una prescripción abierta. Finaliza la sesión de trabajo.",
      inputSchema: PrescriptionCloseSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("prescription_close", input)),
  );

  server.registerTool(
    "prescription_read",
    {
      description: "Lee los detalles de una prescripción por su ID.",
      inputSchema: PrescriptionReadSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("prescription_read", input)),
  );

  server.registerTool(
    "prescription_discard",
    {
      description: "Descarta una prescripción y hace soft-delete en cascada de todas sus pills.",
      inputSchema: PrescriptionDiscardSchema.shape,
    },
    async (input) => fromExecResult(pillboxExec("prescription_discard", input)),
  );

  server.registerTool(
    "bottle_list",
    {
      description: "Lista todos los bottles (proyectos) registrados en Pillbox.",
      inputSchema: {},
    },
    async () => fromExecResult(pillboxExec("bottle_list", {})),
  );
}
