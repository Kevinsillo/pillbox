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
('discovery',            'Descubrimiento no obvio sobre el código o dominio',
 'Incluye: qué se descubrió, dónde, implicaciones',
 'FTS5 tokenizer no normaliza acentos por defecto', 1),
('learning',             'Aprendizaje técnico general',
 'Incluye: qué se aprendió, fuente, cómo aplica',
 'BEGIN IMMEDIATE previene write starvation en WAL mode', 1),
('feedback',             'Corrección o lección aprendida en este proyecto',
 'Incluye: qué no hacer (o qué sí hacer), por qué, y el contexto que motivó la corrección',
 'No usar connection pool aquí — causó corrupción en WAL mode con múltiples writers', 1),
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
('convention',   'Regla, preferencia o convención personal de código',
 'Incluye: la regla o preferencia, dónde aplica y por qué',
 'Snake_case siempre, incluso en TS; prefiero funciones pequeñas con un solo nivel de abstracción', 1),
('workflow',     'Forma de trabajar o proceso personal',
 'Incluye: el flujo, cuándo aplica y por qué funciona para ti',
 'Siempre leer el código antes de modificar; nunca commits directos a main', 1),
('environment',  'Entorno de desarrollo: OS, shell, herramientas, versiones',
 'Incluye: qué usas, versión si es relevante, y cualquier particularidad',
 'Linux + fish shell; Node 22 vía fnm; editor VS Code con vim keybindings', 1),
('context',      'Contexto personal relevante: situación, restricciones, forma de trabajar',
 'Incluye: el contexto, la restricción si aplica, y cómo afecta las decisiones',
 'Proyectos en solitario, priorizo simplicidad; no usar dependencias GPL en proyectos comerciales', 1),
('goal',         'Objetivo personal o de largo plazo',
 'Incluye: el objetivo, motivación y plazo si aplica',
 'Migrar todos los proyectos activos a Pillbox antes de fin de mes', 1),
('feedback',     'Corrección o lección aprendida de una experiencia concreta',
 'Incluye: qué no hacer (o qué sí hacer), por qué, y el contexto que motivó la corrección',
 'No mockear la DB en tests — una migración rota llegó a producción porque los mocks no la detectaron', 1),
('manual',       'Entrada manual sin compound específico', '', NULL, 1);

-- ─── ENTIDADES PRINCIPALES ───────────────────────────────────────────────────

CREATE TABLE bottles (
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
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE prescriptions (
    id         TEXT PRIMARY KEY,
    bottle_id  TEXT NOT NULL,
    -- Título de la tarea/funcionalidad/bug. Obligatorio: el agente DEBE llamar
    -- a prescription_open con título ANTES de insertar cualquier pill.
    -- Flujo: prescription_open → pill_store (N veces) → prescription_close.
    -- El resumen de la sesión se guarda como pill con compound=prescription_summary,
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
    FOREIGN KEY (bottle_id) REFERENCES bottles(id)
);

CREATE INDEX idx_rx_bottle  ON prescriptions(bottle_id);
CREATE INDEX idx_rx_started ON prescriptions(started_at DESC);
-- Garantiza que solo puede haber una prescripción abierta (no cerrada ni descartada) por bottle.
CREATE UNIQUE INDEX idx_rx_open ON prescriptions(bottle_id)
    WHERE ended_at IS NULL AND deleted_at IS NULL;

CREATE TABLE pills (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id         TEXT NOT NULL UNIQUE,
    compound        TEXT NOT NULL,
    title           TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 255),
    content         TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 5000),
    prescription_id TEXT NOT NULL,
    author_name     TEXT,
    author_email    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at      TEXT,
    FOREIGN KEY (compound)        REFERENCES pill_compounds(id),
    FOREIGN KEY (prescription_id) REFERENCES prescriptions(id)
);

CREATE INDEX idx_pill_compound ON pills(compound)        WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_rx       ON pills(prescription_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pill_created  ON pills(created_at DESC);
CREATE UNIQUE INDEX idx_pill_sync ON pills(sync_id);

CREATE TABLE capsules (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id    TEXT NOT NULL UNIQUE,
    compound   TEXT NOT NULL,
    title      TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 255),
    content    TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 5000),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    FOREIGN KEY (compound) REFERENCES capsule_compounds(id)
);

CREATE INDEX idx_cap_compound ON capsules(compound)  WHERE deleted_at IS NULL;
CREATE INDEX idx_cap_created  ON capsules(created_at DESC);
CREATE UNIQUE INDEX idx_cap_sync ON capsules(sync_id);

-- ─── FTS5 ────────────────────────────────────────────────────────────────────

CREATE VIRTUAL TABLE pills_fts USING fts5(
    title, content, compound,
    content='pills',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER pills_ai AFTER INSERT ON pills BEGIN
    INSERT INTO pills_fts(rowid, title, content, compound)
    VALUES (new.id, new.title, new.content, new.compound);
END;

CREATE TRIGGER pills_ad AFTER DELETE ON pills BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound)
    VALUES ('delete', old.id, old.title, old.content, old.compound);
END;

CREATE TRIGGER pills_au AFTER UPDATE ON pills BEGIN
    INSERT INTO pills_fts(pills_fts, rowid, title, content, compound)
    VALUES ('delete', old.id, old.title, old.content, old.compound);
    INSERT INTO pills_fts(rowid, title, content, compound)
    VALUES (new.id, new.title, new.content, new.compound);
END;

CREATE VIRTUAL TABLE capsules_fts USING fts5(
    title, content, compound,
    content='capsules',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER capsules_ai AFTER INSERT ON capsules BEGIN
    INSERT INTO capsules_fts(rowid, title, content, compound)
    VALUES (new.id, new.title, new.content, new.compound);
END;

CREATE TRIGGER capsules_ad AFTER DELETE ON capsules BEGIN
    INSERT INTO capsules_fts(capsules_fts, rowid, title, content, compound)
    VALUES ('delete', old.id, old.title, old.content, old.compound);
END;

CREATE TRIGGER capsules_au AFTER UPDATE ON capsules BEGIN
    INSERT INTO capsules_fts(capsules_fts, rowid, title, content, compound)
    VALUES ('delete', old.id, old.title, old.content, old.compound);
    INSERT INTO capsules_fts(rowid, title, content, compound)
    VALUES (new.id, new.title, new.content, new.compound);
END;

-- ─── REGISTRY DE BOTTLES LOCALES (solo en DB global) ────────────────────────
-- Esta tabla solo tiene sentido en ~/.pillbox/pillbox.db.
-- En DBs locales existe pero permanece vacía.

CREATE TABLE registered_bottles (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    bottle_id     TEXT NOT NULL,             -- UUID del bottle en la DB local
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
