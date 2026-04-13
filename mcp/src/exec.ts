/**
 * Spawner de `pillbox exec`.
 *
 * Envía un payload JSON por stdin al subproceso y devuelve el JSON de stdout.
 * El binario siempre responde con { ok, data?, error?, message? }.
 */

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

// ─── Localización del binario ─────────────────────────────────────────────────

function pillboxBin(): string {
  if (process.env.PILLBOX_BIN) {
    return resolve(process.env.PILLBOX_BIN);
  }
  // En producción el binario está en PATH
  return "pillbox";
}

// ─── Tipos de respuesta del protocolo exec ────────────────────────────────────

export interface ExecOk<T = unknown> {
  ok: true;
  data: T;
}

export interface ExecErr {
  ok: false;
  error: string;
  message: string;
  data?: unknown;
}

export type ExecResult<T = unknown> = ExecOk<T> | ExecErr;

// ─── Función principal ────────────────────────────────────────────────────────

export function pillboxExec<T = unknown>(
  tool: string,
  input: Record<string, unknown>,
): ExecResult<T> {
  const payload = JSON.stringify({ tool, input });

  const result = spawnSync(pillboxBin(), ["exec"], {
    input: payload,
    encoding: "utf8",
    timeout: 15_000,
  });

  if (result.error) {
    return {
      ok: false,
      error: "exec_spawn_error",
      message: result.error.message,
    };
  }

  if (result.status !== 0) {
    const stderr = result.stderr?.trim() ?? "";
    return {
      ok: false,
      error: "exec_nonzero_exit",
      message: stderr || `pillbox exec salió con código ${result.status}`,
    };
  }

  const stdout = result.stdout?.trim() ?? "";
  if (!stdout) {
    return {
      ok: false,
      error: "exec_empty_output",
      message: "pillbox exec no produjo salida",
    };
  }

  try {
    return JSON.parse(stdout) as ExecResult<T>;
  } catch {
    return {
      ok: false,
      error: "exec_parse_error",
      message: `No se pudo parsear la respuesta: ${stdout.slice(0, 200)}`,
    };
  }
}
