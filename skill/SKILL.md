# Pillbox — Memoria persistente para agentes IA

Pillbox guarda el conocimiento generado durante el trabajo en proyectos para que puedas recuperarlo en sesiones futuras. Piensa en él como un diario técnico estructurado que el agente escribe y lee.

---

## Cuándo usar Pillbox

**Al inicio de cada sesión** — recupera el contexto del proyecto:
```
pill_context(bottle_id: <id>)
```
Devuelve un resumen Markdown con prescripciones recientes y sus pills. Úsalo siempre antes de empezar a trabajar en un proyecto conocido.

**Durante el trabajo** — guarda lo que descubres, decides o aprendes:
```
prescription_open → pill_take (varias veces) → prescription_close
```

**Para buscar** — cuando necesitas recordar algo específico:
```
pill_find(q: "auth middleware", bottle_id: <id>)
```

---

## Pills vs Capsules

| | Pills | Capsules |
|---|---|---|
| **Scope** | Un proyecto (bottle) | Cross-proyecto (global) |
| **Ciclo** | Dentro de una prescripción | Independiente |
| **Qué guardar** | Decisiones, bugs, patrones del proyecto | Preferencias, workflow, entorno del usuario |

**Usa pills para**: decisiones de arquitectura, bugs resueltos, patrones establecidos, descubrimientos sobre el código.

**Usa capsules para**: convenciones personales, forma de trabajar, entorno de desarrollo, objetivos.

---

## Flujo de trabajo normal

### 1. Inicio de sesión
```
bottle_list()                          → encontrar el bottle_id del proyecto
pill_context(bottle_id: X)             → recuperar contexto reciente
capsule_find(query: "convenciones")    → recordar preferencias personales relevantes
```

### 2. Durante el trabajo
```
prescription_open(bottle_id: X, title: "Implementar autenticación JWT")
  → devuelve { id: "uuid-de-la-rx" }

pill_take(prescription_id: "...", compound: "decision", title: "...", content: "...")
pill_take(prescription_id: "...", compound: "bugfix", title: "...", content: "...")
...

prescription_close(id: "uuid-de-la-rx")
```

### 3. Resumen antes de cerrar
Antes de `prescription_close`, guarda un resumen:
```
pill_take(
  prescription_id: "...",
  compound: "prescription_summary",
  title: "Resumen: Implementar autenticación JWT",
  content: "## Objetivo\n...\n## Logrado\n...\n## Próximos pasos\n..."
)
```

---

## Compounds de pills

| Compound | Cuándo usarlo |
|---|---|
| `decision` | Elección técnica o de diseño: qué, por qué, qué se descartó |
| `architecture` | Estructura, diseño de sistema o componentes |
| `bugfix` | Bug resuelto: síntoma, causa raíz, fix aplicado |
| `pattern` | Patrón o convención establecida en este proyecto |
| `discovery` | Algo no obvio encontrado en el código o dominio |
| `learning` | Aprendizaje técnico (el modelo falló, reintentó, extrajo lección) |
| `feedback` | Corrección o lección aprendida en este proyecto concreto |
| `prescription_summary` | Resumen de la sesión — siempre al cerrar |
| `manual` | Entrada libre sin categoría específica |

## Compounds de capsules

| Compound | Cuándo usarlo |
|---|---|
| `convention` | Regla o preferencia de código del usuario |
| `workflow` | Proceso o forma de trabajar |
| `environment` | OS, shell, herramientas, versiones |
| `context` | Situación personal, restricciones, forma de trabajar |
| `goal` | Objetivo personal o de largo plazo |
| `feedback` | Lección aprendida de una experiencia concreta |
| `manual` | Entrada libre |

---

## Gestión de prescripciones abiertas

Si `prescription_open` devuelve error `prescription_already_open`, el campo `data` contiene la prescripción existente:
```json
{
  "error": "prescription_already_open",
  "data": { "id": "...", "title": "...", "started_at": "...", "pill_count": 3 }
}
```

Opciones:
- **Reutilizar**: pasa el `id` existente directamente a `pill_take`
- **Cerrar y abrir nueva**: `prescription_close(id: "...")` → `prescription_open(...)`

---

## Referencia rápida de tools

```
# Bottles
bottle_list()
bottle_create(name, display_name, directory, scope)

# Prescriptions
prescription_open(bottle_id, title)
prescription_close(id)
prescription_read(id)
prescription_discard(id)

# Pills
pill_take(prescription_id, compound, title, content, [dispenser], [author_name], [author_email])
pill_read(id)
pill_revise(id, patch: {title?, content?})
pill_discard(id)
pill_find(q, [bottle_id], [compound], [limit])
pill_context(bottle_id, [prescription_limit=5], [pill_limit=30])

# Capsules
capsule_take(compound, title, content, [dispenser])
capsule_read(id)
capsule_revise(id, patch: {title?, content?})
capsule_discard(id)
capsule_find(query, [compound], [limit])

# Admin
stats()
```
