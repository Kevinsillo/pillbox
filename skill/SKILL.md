---
name: pillbox
description: >
  Apoyo para las MCP tools de Pillbox — memoria persistente por proyectos para agentes IA.
  Usar cuando las herramientas pill_take, pill_search, prescription_open, bottle_list,
  capsule_take o capsule_search están disponibles,
  al empezar trabajo en un proyecto conocido, o cuando el usuario pide recordar o recuperar algo.
metadata:
  version: 3.0.0
---

# Pillbox

Dos tipos de memoria independientes. La elección entre ellos es la decisión más importante.

| | **Pills** | **Capsules** |
|---|---|---|
| ¿Qué guarda? | Conocimiento del proyecto actual | Preferencias y hábitos del usuario |
| ¿Requiere prescription? | Sí — siempre | No |
| ¿Scope? | Un bottle (proyecto) | Global, cross-proyecto |
| Ejemplo | "decidimos usar UUID v7" | "el usuario prefiere commits en español" |

---

## Arranque de sesión

```json
{ "tool": "capsule_search", "query": "<términos relevantes>" }
{ "tool": "bottle_list" }
{ "tool": "pill_context", "bottle_id": 1 }
{ "tool": "prescription_open", "bottle_id": 1, "title": "<descripción de la tarea>" }
```

Si `prescription_open` devuelve `prescription_already_open`: el campo `data` contiene la prescripción
activa — reutilizar ese `id` directamente sin abrir otra.

---

## Cierre de sesión

```json
{ "tool": "pill_take", "prescription_id": "<uuid>", "compound": "prescription_summary", "title": "<título>", "content": "<resumen>" }
{ "tool": "prescription_close", "id": "<uuid>" }
```

`prescription_summary` es **obligatorio** antes de cerrar. Sin él se pierde el contexto de la sesión.

---

## Tools completas

### Pills

| Tool | Parámetros clave | Cuándo |
|---|---|---|
| `pill_take` | `prescription_id`, `compound`, `title`, `content` | Guardar conocimiento nuevo |
| `pill_search` | `query`, `bottle_id?`, `compound?`, `limit?` | Buscar antes de crear (evitar duplicados) |
| `pill_context` | `bottle_id`, `prescription_limit?`, `pill_limit?` | Cargar contexto al inicio |
| `pill_read` | `id` | Leer contenido completo de una pill |
| `pill_revise` | `id`, `patch{title?, content?}` | Actualizar pill existente |
| `pill_discard` | `id` | Soft-delete (irreversible) |

### Capsules

| Tool | Parámetros clave | Cuándo |
|---|---|---|
| `capsule_take` | `compound`, `title`, `content` | Guardar preferencia/hábito del usuario |
| `capsule_search` | `query`, `compound?`, `limit?` | Buscar preferencias al inicio o antes de crear |
| `capsule_read` | `id` | Leer contenido completo |
| `capsule_revise` | `id`, `patch{title?, content?, compound?}` | Actualizar capsule existente |
| `capsule_discard` | `id` | Soft-delete |

### Prescriptions

| Tool | Parámetros clave | Cuándo |
|---|---|---|
| `prescription_open` | `bottle_id`, `title` | Iniciar sesión de trabajo |
| `prescription_close` | `id` | Finalizar sesión |
| `prescription_read` | `id` | Ver detalles de una prescripción |
| `prescription_discard` | `id` | Eliminar prescripción + todas sus pills en cascada |

### Bottles

| Tool | Parámetros clave | Cuándo |
|---|---|---|
| `bottle_list` | — | Listar proyectos registrados |
| `bottle_create` | `name`, `display_name`, `directory`, `scope` | Registrar proyecto nuevo (normalmente lo hace el CLI) |
| `stats` | — | Alias de `bottle_list` |
| `pill_compounds` | — | Compounds disponibles para pill_take |
| `capsule_compounds` | — | Compounds disponibles para capsule_take |

---

## Compounds

Los compounds son dinámicos. Consultar antes de elegir:
- `pill_compounds` → lista de compounds válidos para `pill_take`
- `capsule_compounds` → lista de compounds válidos para `capsule_take`

Cada entry incluye `id`, `description` y `prompt_hint` con instrucciones de formato.

---

## Formato de contenido

Las pills las lee una IA, no un humano. Máxima densidad, mínimos tokens.
Target: 100–400 chars. El límite es 5000 — es un techo, no un objetivo.

```
# decision / architecture / bugfix
symptom/context: una línea
chosen/fix: qué y por qué
discarded: alternativas descartadas (si las hay)

# pattern / discovery / learning
Prosa técnica densa, 1-2 frases. Sin cabeceras.

# prescription_summary
goal: una línea
done: bullet por item logrado
found: descubrimientos no obvios (omitir si ninguno)
next: pendiente
files: solo los modificados significativamente
```

---

## Reglas

- **Buscar antes de crear**: `pill_search` / `capsule_search` antes de `pill_take` / `capsule_take` para evitar duplicados.
- **No guardar lo que está en el código**: solo lo que no es obvio leyendo el repo (decisiones, contexto, causas).
- **prescription_summary siempre**: sin él, el contexto de la sesión se pierde para futuras sesiones.
- **Subagentes no usan MCP**: solo el orquestrador llama `pill_take`. Los subagentes devuelven hallazgos estructurados y el orquestrador consolida antes de guardar.
