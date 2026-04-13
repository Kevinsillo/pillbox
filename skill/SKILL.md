---
name: pillbox
description: >
  Apoyo para las MCP tools de Pillbox — memoria persistente por proyectos para agentes IA.
  Usar cuando las herramientas pill_take, pill_find, prescription_open o bottle_list están disponibles,
  al empezar trabajo en un proyecto conocido, o cuando el usuario pide recordar o recuperar algo.
metadata:
  version: 2.0.0
---

# Pillbox — Referencia MCP

Pillbox guarda conocimiento generado durante el trabajo para recuperarlo en sesiones futuras.

## Flujo básico

```
bottle_list()                          → encontrar el bottle_id del proyecto actual
pill_context(bottle_id)                → recuperar contexto antes de empezar
prescription_open(bottle_id, title)    → abrir sesión con título descriptivo de la tarea
  pill_take(prescription_id, ...)      → guardar decisiones, bugs, patrones durante el trabajo
  pill_take(compound: "prescription_summary", ...)  → resumen antes de cerrar
prescription_close(id)                 → cerrar al terminar
```

## Pills vs Capsules

| | Pills | Capsules |
|---|---|---|
| **Scope** | Un proyecto (dentro de una prescripción) | Cross-proyecto (globales) |
| **Qué guardar** | Decisiones, bugs, patrones del código | Preferencias, workflow, entorno del usuario |

## Compounds — Pills

| Compound | Cuándo |
|---|---|
| `decision` | Elección técnica: qué, por qué, qué se descartó |
| `architecture` | Estructura, diseño de sistema o módulos |
| `bugfix` | Bug resuelto: síntoma, causa raíz, fix |
| `pattern` | Convención establecida en este proyecto |
| `discovery` | Algo no obvio encontrado en el código o dominio |
| `learning` | El modelo falló y extrajo una lección |
| `feedback` | El usuario corrigió el enfoque del modelo |
| `prescription_summary` | Resumen de sesión — siempre antes de cerrar |

## Compounds — Capsules

| Compound | Cuándo |
|---|---|
| `convention` | Preferencia de estilo o naming que aplica a todo |
| `workflow` | Proceso preferido del usuario |
| `environment` | OS, shell, herramientas, versiones |
| `context` | Restricciones personales o situación del equipo |
| `goal` | Objetivo de largo plazo del usuario |

## Error: prescription_already_open

Si `prescription_open` devuelve este error, el campo `data` contiene la prescripción activa.
- **Reutilizar**: pasar el `id` existente a `pill_take`
- **Cerrar y nueva**: `prescription_close(id)` → `prescription_open(...)`
