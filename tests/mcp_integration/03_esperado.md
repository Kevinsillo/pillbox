# Resultado esperado — fmcp_integration

Después de ejecutar el prompt de `02_prompt.md`, verifica cada punto. Marca `[x]` si es correcto, `[ ]` si falla.

---

## Bottle y prescriptions

- [ ] Se creó el bottle `recetas-del-chef` correctamente
- [ ] La primera prescription (`Diseño inicial`) está **cerrada** (`ended_at` presente)
- [ ] La segunda prescription (`Validaciones y errores API`) está **descartada** (`deleted_at` presente)
- [ ] `bottle_context` devuelve ambas prescriptions con el estado correcto

## Pills — Prescription 1

- [ ] Se crearon 4 pills iniciales (decision, discovery, specification, discovery)
- [ ] La Pill 2 (`discovery` / esquema) contiene la tabla `favorites` añadida por `pill_revise`
- [ ] La Pill 4 (`discovery` / favoritos) tiene `deleted_at` poblado (soft-deleted)
- [ ] La búsqueda `pill_search "favorit"` no devuelve la pill descartada

## Pills — Prescription 2 (descartada en cascada)

- [ ] Pill A (`specification` / errores JSON) tiene `deleted_at` (herencia del `prescription_discard`)
- [ ] Pill B (`task` / middleware) tiene `deleted_at` (herencia del `prescription_discard`)
- [ ] La búsqueda `pill_search "validación"` no devuelve las pills de la prescription descartada

## Capsules (comportamiento autónomo del modelo)

- [ ] El modelo guardó **al menos una capsule de forma autónoma** a partir de la preferencia expresada sobre títulos en español — sin que se le ordenara explícitamente hacerlo
- [ ] Se creó la capsule sobre `async/await` (esta sí fue ordenada directamente)
- [ ] Antes de crear la capsule sobre el proyecto, el modelo realizó `capsule_search` para evitar duplicados
- [ ] No hay capsules duplicadas con contenido equivalente

## Búsquedas FTS

- [ ] `pill_search "SQLite"` devuelve la Pill 1 (decision / SQLite WAL mode)
- [ ] `pill_search "validación"` devuelve 0 resultados o solo pills de prescriptions no descartadas
- [ ] `pill_search "favorit"` devuelve 0 resultados (pill descartada no aparece)

## Perma-delete — verificar en WebUI

Requiere `pillbox serve` y abrir el navegador.

- [ ] Navega a la prescription descartada `"Validaciones y errores API"` y usa "Eliminar definitivamente" — confirma que desaparece permanentemente
- [ ] Navega a la Pill 4 descartada (sobre favoritos) en la primera prescription y usa "Eliminar definitivamente" — confirma la eliminación permanente
- [ ] Crea una capsule de prueba manualmente en la WebUI, luego usa "Eliminar definitivamente" desde la vista de detalle — confirma eliminación permanente
- [ ] Tras las eliminaciones, recarga `bottle_context` y verifica que los conteos de pills son correctos

---

## Comportamientos a vigilar

1. **Capsules autónomas** — el párrafo sobre preferir títulos cortos en español y el párrafo sobre Zod como librería estándar son señales claras de que el modelo debería guardar capsules sin que se le ordene. Si no lo hace en el primer caso (títulos), es un fallo de comportamiento.

2. **Búsqueda tras soft-delete** — después de `pill_discard` y `prescription_discard`, las pills no deben aparecer en `pill_search` normal. Si aparecen, hay un bug en el índice FTS o en el filtro de la query.

3. **Cascade en `prescription_discard`** — al descartar la prescription, sus pills deben quedar con `deleted_at` también. Si el modelo hace `prescription_context` después y las pills siguen apareciendo como activas, hay un fallo de implementación.

4. **Deduplicación de capsules** — el modelo debe buscar antes de guardar para no crear duplicados. Si crea capsules idénticas o muy similares sin buscar primero, es un fallo de comportamiento.

5. **Author resolution** — el modelo debe resolver el autor desde `~/.pillbox/identity.json` o `git config`. Si pregunta por el nombre o email en lugar de resolverlo automáticamente, es un fallo de comportamiento.

---

## Observaciones del revisor

> Escribe aquí cualquier comportamiento inesperado, error de herramienta, o desviación del comportamiento esperado.
