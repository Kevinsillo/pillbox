-- Schema inicial — DB LOCAL (proyecto)
-- Tablas: bottles, prescriptions, pills, pills_fts, schema_migrations.
-- NO incluye: capsules, capsules_fts, registered_bottles (esos viven solo en la DB global).

-- ─── PRAGMA ──────────────────────────────────────────────────────────────────

PRAGMA journal_mode  = WAL;
PRAGMA busy_timeout  = 5000;
PRAGMA synchronous   = NORMAL;
PRAGMA foreign_keys  = ON;
PRAGMA auto_vacuum   = INCREMENTAL;

-- ─── ENTIDADES PRINCIPALES ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS bottles (
    id           TEXT PRIMARY KEY,              -- UUID v7, generado en Rust antes del INSERT
    -- Slug generado automáticamente del nombre de carpeta (lowercase, normalizado).
    -- Inmutable: identidad técnica del bottle. No lo elige el usuario.
    name         TEXT NOT NULL UNIQUE,
    -- Nombre legible asignado por el usuario en `pillbox bottle init`.
    -- El wizard DEBE preguntar: "¿Cómo quieres llamar a este proyecto?"
    display_name TEXT NOT NULL,
    -- Ruta absoluta al directorio del proyecto. Permite detectar si el proyecto
    -- se movió de directorio (mismo name, distinto directory).
    directory    TEXT NOT NULL UNIQUE,
    scope        TEXT NOT NULL DEFAULT 'local' CHECK (scope IN ('local', 'global')),
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    views        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS prescriptions (
    id         TEXT PRIMARY KEY,
    bottle_id  TEXT NOT NULL,
    -- Título de la tarea/funcionalidad/bug. Obligatorio: el agente DEBE llamar
    -- a prescription_open con título ANTES de insertar cualquier pill.
    -- Flujo: prescription_open → pill_store (N veces) → prescription_close.
    -- El resumen de la sesión se guarda como pill con compound=summary,
    -- no como campo aquí — así es searchable vía FTS5 y vive en el timeline.
    title      TEXT NOT NULL,
    -- Autoría de la prescription: nombre y email del usuario humano detrás del agente.
    -- Resolución (responsabilidad del MCP/cliente): (1) git config user.name/user.email,
    -- (2) ~/.pillbox/identity.json (campos name/email), (3) preguntar al usuario y persistir
    -- en ese fichero. Pueden ser NULL si no se ha resuelto.
    author_name  TEXT,
    author_email TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at   TEXT,
    -- Soft delete: al descartar una prescripción, sus pills también se soft-deletan
    -- en el store (cascade lógico en Rust, no a nivel DB).
    deleted_at TEXT,
    views      INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (bottle_id) REFERENCES bottles(id)
);

CREATE INDEX IF NOT EXISTS idx_rx_bottle  ON prescriptions(bottle_id);
CREATE INDEX IF NOT EXISTS idx_rx_started ON prescriptions(started_at DESC);

CREATE TABLE IF NOT EXISTS pills (
    id              TEXT PRIMARY KEY,              -- UUID v7, generado en Rust antes del INSERT
    compound        TEXT NOT NULL,
    title           TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 255),
    content         TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 5000),
    prescription_id TEXT NOT NULL,
    author_name     TEXT,
    author_email    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at      TEXT,
    views           INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (prescription_id) REFERENCES prescriptions(id)
);

CREATE INDEX IF NOT EXISTS idx_pill_compound ON pills(compound)        WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_pill_rx       ON pills(prescription_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_pill_created  ON pills(created_at DESC);

-- ─── FTS5 ────────────────────────────────────────────────────────────────────

CREATE VIRTUAL TABLE IF NOT EXISTS pills_fts USING fts5(
    title, content, compound,
    content='pills',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS pills_ai AFTER INSERT ON pills BEGIN
    INSERT INTO pills_fts(rowid, title, content, compound)
    VALUES (new.rowid, new.title, new.content, new.compound);
END;

CREATE TRIGGER IF NOT EXISTS pills_ad AFTER DELETE ON pills BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound)
    VALUES ('delete', old.rowid, old.title, old.content, old.compound);
END;

CREATE TRIGGER IF NOT EXISTS pills_au AFTER UPDATE ON pills
WHEN OLD.title <> NEW.title OR OLD.content <> NEW.content OR OLD.compound <> NEW.compound
BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound)
    VALUES ('delete', old.rowid, old.title, old.content, old.compound);
    INSERT INTO pills_fts(rowid, title, content, compound)
    VALUES (new.rowid, new.title, new.content, new.compound);
END;

-- ─── VERSIÓN DE SCHEMA ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS schema_migrations (
    version    INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (1, 'initial_local');
