/**
 * Schemas Zod para los inputs de todas las tools.
 * Se usan tanto para validación como para generar el inputSchema del MCP.
 */

import { z } from "zod";

// ─── Pills ────────────────────────────────────────────────────────────────────

export const PillTakeSchema = z.object({
  prescription_id: z.string().uuid(),
  compound: z.enum([
    "decision",
    "architecture",
    "bugfix",
    "pattern",
    "discovery",
    "learning",
    "feedback",
    "prescription_summary",
    "manual",
  ]),
  title: z.string().min(1).max(200),
  content: z.string().min(1),
  dispenser: z.string().max(100).optional(),
  author_name: z.string().max(100).optional(),
  author_email: z.string().max(200).optional(),
});

export const PillReadSchema = z.object({
  id: z.number().int().positive(),
});

export const PillReviseSchema = z.object({
  id: z.number().int().positive(),
  patch: z.object({
    title: z.string().min(1).max(200).optional(),
    content: z.string().min(1).optional(),
  }),
});

export const PillDiscardSchema = z.object({
  id: z.number().int().positive(),
});

export const PillFindSchema = z.object({
  query: z.string().min(1),
  bottle_id: z.number().int().positive().optional(),
  compound: z
    .enum([
      "decision",
      "architecture",
      "bugfix",
      "pattern",
      "discovery",
      "learning",
      "feedback",
      "prescription_summary",
      "manual",
    ])
    .optional(),
  limit: z.number().int().min(1).max(100).optional(),
});

export const PillContextSchema = z.object({
  bottle_id: z.number().int().positive(),
  prescription_limit: z.number().int().min(1).max(20).optional(),
  pill_limit: z.number().int().min(1).max(100).optional(),
});

// ─── Capsules ─────────────────────────────────────────────────────────────────

export const CapsuleTakeSchema = z.object({
  compound: z.enum([
    "convention",
    "workflow",
    "environment",
    "context",
    "goal",
    "feedback",
    "manual",
  ]),
  title: z.string().min(1).max(200),
  content: z.string().min(1),
  dispenser: z.string().max(100).optional(),
});

export const CapsuleReadSchema = z.object({
  id: z.number().int().positive(),
});

export const CapsuleReviseSchema = z.object({
  id: z.number().int().positive(),
  patch: z.object({
    title: z.string().min(1).max(200).optional(),
    content: z.string().min(1).optional(),
    compound: z
      .enum(["convention", "workflow", "environment", "context", "goal", "feedback", "manual"])
      .optional(),
  }),
});

export const CapsuleDiscardSchema = z.object({
  id: z.number().int().positive(),
});

export const CapsuleFindSchema = z.object({
  query: z.string().min(1),
  compound: z
    .enum(["convention", "workflow", "environment", "context", "goal", "feedback", "manual"])
    .optional(),
  limit: z.number().int().min(1).max(100).optional(),
});

// ─── Prescriptions ────────────────────────────────────────────────────────────

export const PrescriptionOpenSchema = z.object({
  bottle_id: z.number().int().positive(),
  title: z.string().min(1).max(300),
});

export const PrescriptionCloseSchema = z.object({
  id: z.string().uuid(),
});

export const PrescriptionReadSchema = z.object({
  id: z.string().uuid(),
});

export const PrescriptionDiscardSchema = z.object({
  id: z.string().uuid(),
});

// ─── Bottles ──────────────────────────────────────────────────────────────────

export const BottleCreateSchema = z.object({
  name: z.string().min(1).max(100),
  display_name: z.string().min(1).max(200),
  directory: z.string().min(1),
  scope: z.enum(["local", "global"]),
});
