-- Fase 1: Schema inicial completo de Pillbox
-- Orden: tablas de referencia primero (requeridas por FK)

-- ─── PRAGMA ──────────────────────────────────────────────────────────────────

PRAGMA journal_mode  = WAL;
PRAGMA busy_timeout  = 5000;
PRAGMA synchronous   = NORMAL;
PRAGMA foreign_keys  = ON;
PRAGMA auto_vacuum   = INCREMENTAL;

-- ─── TABLAS DE REFERENCIA ────────────────────────────────────────────────────

CREATE TABLE pill_compounds (
    id            TEXT PRIMARY KEY,
    description   TEXT NOT NULL,
    prompt_hint   TEXT NOT NULL,
    example_title TEXT,
    is_active     INTEGER NOT NULL DEFAULT 1
);

INSERT INTO pill_compounds VALUES
('decision',             'Decisión técnica o de diseño tomada',
 'Incluye: contexto, opciones consideradas, opción elegida y por qué',
 'Elegido UUID v7 sobre ULID para sync_id', 1),
('architecture',         'Estructura, diseño de sistema o componentes',
 'Incluye: qué se diseñó, cómo funciona, por qué esta estructura',
 'Auth usa JWT stateless con refresh tokens en SQLite', 1),
('bugfix',               'Bug corregido con causa raíz',
 'Incluye: síntoma, causa raíz, fix aplicado, archivos modificados',
 'Race condition en dedup — resuelto con BEGIN IMMEDIATE', 1),
('pattern',              'Patrón o convención establecida en el proyecto',
 'Incluye: el patrón, dónde aplica, por qué se adoptó',
 'Todos los handlers devuelven Result<Json<Response>, AppError>', 1),
('config',               'Configuración relevante del proyecto',
 'Incluye: qué se configuró, valor, razón',
 'PRAGMA journal_mode = WAL para lecturas concurrentes', 1),
('discovery',            'Descubrimiento no obvio sobre el código o dominio',
 'Incluye: qué se descubrió, dónde, implicaciones',
 'FTS5 tokenizer no normaliza acentos por defecto', 1),
('learning',             'Aprendizaje técnico general',
 'Incluye: qué se aprendió, fuente, cómo aplica',
 'BEGIN IMMEDIATE previene write starvation en WAL mode', 1),
('prescription_summary', 'Resumen de prescription de trabajo',
 'Incluye: objetivo, logrado, descubrimientos, próximos pasos, archivos relevantes',
 NULL, 1),
('manual',               'Entrada manual sin compound específico', '', NULL, 1);

CREATE TABLE capsule_compounds (
    id            TEXT PRIMARY KEY,
    description   TEXT NOT NULL,
    prompt_hint   TEXT NOT NULL,
    example_title TEXT,
    is_active     INTEGER NOT NULL DEFAULT 1
);

INSERT INTO capsule_compounds VALUES
('preference',  'Preferencia personal de desarrollo',
 'Incluye: la preferencia y por qué',
 'Prefiero funciones pequeñas con un solo nivel de abstracción', 1),
('convention',  'Convención personal de código',
 'Incluye: la convención y contexto',
 'Nombres de variables en snake_case siempre, incluso en TS', 1),
('workflow',    'Forma de trabajar o proceso personal',
 'Incluye: el flujo y cuándo aplica',
 'Siempre leer el código antes de modificar', 1),
('skill',       'Habilidad o conocimiento técnico del usuario',
 'Incluye: qué sé, nivel, contexto de uso',
 'Rust: nivel intermedio, Go: nivel avanzado', 1),
('context',     'Contexto personal relevante para el trabajo',
 'Incluye: el contexto y cómo afecta las decisiones',
 'Trabajo en proyectos solista, priorizo simplicidad sobre escalabilidad', 1),
('goal',        'Objetivo personal o de largo plazo',
 'Incluye: el objetivo y plazo si aplica',
 'Migrar todos los proyectos activos a Pillbox este mes', 1),
('constraint',  'Restricción o limitación a tener en cuenta',
 'Incluye: la restricción y por qué existe',
 'No usar dependencias con licencia GPL en proyectos comerciales', 1),
('manual',      'Entrada manual sin compound específico', '', NULL, 1);

CREATE TABLE dispenser_types (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL
);

INSERT INTO dispenser_types VALUES
('pill_take',   'Herramienta MCP pill_take'),
('pill_revise', 'Herramienta MCP pill_revise'),
('api',         'Llamada directa a la HTTP API'),
('cli',         'Comando CLI pillbox');

CREATE TABLE action_types (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL
);

INSERT INTO action_types VALUES
('pill_take',           'Pill creada o actualizada'),
('pill_revise',         'Pill revisada'),
('pill_discard',        'Pill eliminada (soft delete)'),
('pill_find',           'Búsqueda de pills'),
('capsule_take',        'Capsule creada o actualizada'),
('capsule_revise',      'Capsule revisada'),
('capsule_discard',     'Capsule eliminada'),
('prescription_open',   'Prescription abierta'),
('prescription_close',  'Prescription cerrada');

CREATE TABLE link_types (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL
);

INSERT INTO link_types VALUES
('caused',     'Esta pill causó o motivó la otra'),
('fixes',      'Esta pill corrige un problema descrito en la otra'),
('supersedes', 'Esta pill reemplaza o invalida la otra'),
('related',    'Relación temática sin jerarquía'),
('implements', 'Esta pill implementa una decisión descrita en la otra');

-- ─── IDENTIDADES ─────────────────────────────────────────────────────────────

CREATE TABLE identities (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    email      TEXT,
    source     TEXT NOT NULL CHECK (source IN ('git', 'system', 'manual')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (name, email)
);

-- ─── ENTIDADES PRINCIPALES ───────────────────────────────────────────────────

CREATE TABLE bottles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    db_scope     TEXT NOT NULL DEFAULT 'local' CHECK (db_scope IN ('local', 'global')),
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE prescriptions (
    id          TEXT PRIMARY KEY,
    bottle      TEXT NOT NULL,
    directory   TEXT NOT NULL,
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at    TEXT,
    summary     TEXT,
    FOREIGN KEY (bottle) REFERENCES bottles(name)
);

CREATE INDEX idx_rx_bottle  ON prescriptions(bottle);
CREATE INDEX idx_rx_started ON prescriptions(started_at DESC);

CREATE TABLE pills (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id         TEXT NOT NULL UNIQUE,
    compound        TEXT NOT NULL,
    title           TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 255),
    content         TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 5000),
    bottle          TEXT NOT NULL,
    prescription_id TEXT NOT NULL,
    dispenser       TEXT,
    formula         TEXT CHECK (
                        formula IS NULL OR (
                            length(formula) BETWEEN 1 AND 120 AND
                            formula GLOB '[a-z0-9/-]*'
                        )
                    ),
    normalized_hash TEXT NOT NULL,
    dosage          INTEGER NOT NULL DEFAULT 1 CHECK (dosage >= 1),
    duplicate_count INTEGER NOT NULL DEFAULT 1 CHECK (duplicate_count >= 1),
    last_seen_at    TEXT,
    author_name     TEXT,
    author_email    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at      TEXT,
    FOREIGN KEY (compound)        REFERENCES pill_compounds(id),
    FOREIGN KEY (bottle)          REFERENCES bottles(name),
    FOREIGN KEY (prescription_id) REFERENCES prescriptions(id),
    FOREIGN KEY (dispenser)       REFERENCES dispenser_types(id)
);

CREATE INDEX idx_pill_bottle   ON pills(bottle)                             WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_compound ON pills(compound)                           WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_rx       ON pills(prescription_id)                    WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_formula  ON pills(formula, bottle, updated_at DESC)   WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_dedupe   ON pills(normalized_hash, bottle, compound)  WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_created  ON pills(created_at DESC);
CREATE UNIQUE INDEX idx_pill_sync ON pills(sync_id);

CREATE TABLE capsules (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id         TEXT NOT NULL UNIQUE,
    compound        TEXT NOT NULL,
    title           TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 255),
    content         TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 5000),
    formula         TEXT CHECK (
                        formula IS NULL OR (
                            length(formula) BETWEEN 1 AND 120 AND
                            formula GLOB '[a-z0-9/-]*'
                        )
                    ),
    normalized_hash TEXT NOT NULL,
    dosage          INTEGER NOT NULL DEFAULT 1 CHECK (dosage >= 1),
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at      TEXT,
    FOREIGN KEY (compound) REFERENCES capsule_compounds(id)
);

CREATE INDEX idx_cap_compound ON capsules(compound)                  WHERE deleted_at IS NULL;
CREATE INDEX idx_cap_formula  ON capsules(formula, updated_at DESC)  WHERE deleted_at IS NULL;
CREATE INDEX idx_cap_created  ON capsules(created_at DESC);
CREATE UNIQUE INDEX idx_cap_sync ON capsules(sync_id);

CREATE TABLE pill_links (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id    INTEGER NOT NULL,
    to_id      INTEGER NOT NULL,
    rel_type   TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (from_id)  REFERENCES pills(id),
    FOREIGN KEY (to_id)    REFERENCES pills(id),
    FOREIGN KEY (rel_type) REFERENCES link_types(id),
    UNIQUE (from_id, to_id, rel_type)
);

CREATE TABLE dispense_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    prescription_id TEXT,
    bottle          TEXT,
    action          TEXT NOT NULL,
    pill_id         INTEGER,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (prescription_id) REFERENCES prescriptions(id),
    FOREIGN KEY (action)          REFERENCES action_types(id)
);

-- ─── FTS5 ────────────────────────────────────────────────────────────────────

CREATE VIRTUAL TABLE pills_fts USING fts5(
    title, content, compound, bottle, formula,
    content='pills',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER pills_ai AFTER INSERT ON pills BEGIN
    INSERT INTO pills_fts(rowid, title, content, compound, bottle, formula)
    VALUES (new.id, new.title, new.content, new.compound, new.bottle, new.formula);
END;

CREATE TRIGGER pills_ad AFTER DELETE ON pills BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound, bottle, formula)
    VALUES ('delete', old.id, old.title, old.content, old.compound, old.bottle, old.formula);
END;

CREATE TRIGGER pills_au AFTER UPDATE ON pills BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound, bottle, formula)
    VALUES ('delete', old.id, old.title, old.content, old.compound, old.bottle, old.formula);
    INSERT INTO pills_fts(rowid, title, content, compound, bottle, formula)
    VALUES (new.id, new.title, new.content, new.compound, new.bottle, new.formula);
END;

CREATE VIRTUAL TABLE capsules_fts USING fts5(
    title, content, compound, formula,
    content='capsules',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER capsules_ai AFTER INSERT ON capsules BEGIN
    INSERT INTO capsules_fts(rowid, title, content, compound, formula)
    VALUES (new.id, new.title, new.content, new.compound, new.formula);
END;

CREATE TRIGGER capsules_ad AFTER DELETE ON capsules BEGIN
    INSERT INTO capsules_fts(capsules_fts, rowid, title, content, compound, formula)
    VALUES ('delete', old.id, old.title, old.content, old.compound, old.formula);
END;

CREATE TRIGGER capsules_au AFTER UPDATE ON capsules BEGIN
    INSERT INTO capsules_fts(capsules_fts, rowid, title, content, compound, formula)
    VALUES ('delete', old.id, old.title, old.content, old.compound, old.formula);
    INSERT INTO capsules_fts(rowid, title, content, compound, formula)
    VALUES (new.id, new.title, new.content, new.compound, new.formula);
END;

-- ─── REGISTRY DE BOTTLES LOCALES (solo en DB global) ────────────────────────
-- Esta tabla solo tiene sentido en ~/.pillbox/pillbox.db.
-- En DBs locales existe pero permanece vacía.

CREATE TABLE registered_bottles (
    id            TEXT PRIMARY KEY,          -- UUID v7
    name          TEXT NOT NULL,             -- Nombre normalizado del bottle
    display_name  TEXT NOT NULL,             -- Nombre original para mostrar
    db_path       TEXT NOT NULL UNIQUE,      -- Ruta absoluta a la DB local
    registered_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_reg_name ON registered_bottles(name);

-- ─── VERSIÓN DE SCHEMA ───────────────────────────────────────────────────────

CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO schema_migrations (version, name) VALUES (1, 'initial');
