# Resultado esperado — mcp_integration

Después de ejecutar el prompt de `02_prompt.md`, verifica cada punto. Marca `[x]` si es correcto, `[ ]` si falla.

---

## Bottle

- [ ] Se creó un bottle con `scope: local` (default razonable, sin que el usuario lo dijera)
- [ ] El `directory` del bottle coincide con el cwd del proyecto donde se lanzó el test — **no** con `/tmp` ni con ningún path inventado por el modelo
- [ ] `name` y `display_name` se derivaron del proyecto sin pedírselos al usuario

## Prescriptions

- [ ] La primera prescription se creó con un título coherente con "diseño inicial"
- [ ] La primera prescription quedó **cerrada** (`ended_at` presente)
- [ ] La segunda prescription quedó **descartada** (`deleted_at` presente)
- [ ] `bottle_context` devuelve ambas prescriptions con el estado correcto

## Pills — Prescription 1

- [ ] Se crearon 4 pills, **cada una con un compound distinto**, elegido por el modelo según el tipo de información
- [ ] Los títulos están en español y no superan 8 palabras (preferencia declarada al inicio)
- [ ] El contenido es real del proyecto donde se lanzó el test, no inventado ni genérico
- [ ] Una pill fue revisada (`pill_revise`) ampliando contenido — el cambio quedó persistido
- [ ] Una pill fue descartada (`pill_discard`) y tiene `deleted_at` poblado
- [ ] La búsqueda por un término de la pill descartada NO la devuelve

## Pills — Prescription 2 (descartada en cascada)

- [ ] Las pills creadas heredaron `deleted_at` al descartar la prescription
- [ ] No aparecen en `pill_search` normal tras la cascada

## Capsules (comportamiento autónomo del modelo)

- [ ] El modelo guardó capsules de forma **autónoma** a partir de las preferencias declaradas (títulos en español, Zod, async/await) — sin que se le ordenara hacerlo explícitamente
- [ ] Las preferencias NO se guardaron en `MEMORY.md` ni como auto-memory de Claude Code — fueron a `capsule_store`
- [ ] Antes de crear la capsule del proyecto, el modelo hizo `capsule_search` para evitar duplicados
- [ ] No hay capsules duplicadas con contenido equivalente

## Búsquedas FTS

- [ ] Las búsquedas con términos presentes en pills devuelven resultados con título, compound y prescription
- [ ] La búsqueda del término exclusivo de la pill descartada devuelve 0 resultados (o solo pills activas, si el término aparece en otras)
- [ ] Las pills de la prescription descartada en cascada no aparecen

## Perma-delete — verificar en WebUI

Requiere `pillbox serve` y abrir el navegador.

- [ ] Navega a la segunda prescription (descartada) y usa "Eliminar definitivamente" — confirma desaparición permanente
- [ ] Navega a la pill descartada de la primera prescription y usa "Eliminar definitivamente"
- [ ] Crea una capsule manualmente en la WebUI y usa "Eliminar definitivamente" desde la vista de detalle
- [ ] Tras las eliminaciones, recarga `bottle_context` y verifica que los conteos de pills son correctos

---

## Comportamientos a vigilar

1. **Directorio del bottle** — el modelo nunca debe elegir el directorio ni aceptarlo de paths que el usuario suelte en el chat. El `directory` lo deriva el MCP del cwd. Si aparece `/tmp/...` o cualquier path inventado, es un fallo grave.

2. **Pillbox gana a `auto memory`** — las preferencias del usuario deben ir a `capsule_store`, no a `MEMORY.md` / `feedback_*.md`. Si el modelo crea archivos en `~/.claude/projects/.../memory/`, ha caído en el reflejo de la skill `auto memory` y ha ignorado la prioridad de Pillbox.

3. **Capsules autónomas** — las declaraciones del usuario sobre estilo o librerías por defecto deben dispararse como capsules sin orden explícita. Si solo guarda las que se le piden directamente, es un fallo de comportamiento.

4. **Búsqueda tras soft-delete** — tras `pill_discard` y `prescription_discard`, las pills no deben aparecer en `pill_search` normal. Si aparecen, hay un bug en el índice FTS o en el filtro de la query.

5. **Cascade en `prescription_discard`** — al descartar la prescription, sus pills deben quedar con `deleted_at`. Si tras `prescription_context` siguen apareciendo activas, hay un fallo de implementación.

6. **Deduplicación de capsules** — el modelo debe buscar antes de guardar para no crear duplicados.

7. **Author resolution** — el modelo debe resolver el autor desde `~/.pillbox/identity.json` o `git config`. Si pregunta por el nombre o email, es un fallo de comportamiento.

---

## Observaciones del revisor

> Escribe aquí cualquier comportamiento inesperado, error de herramienta, o desviación del comportamiento esperado.
