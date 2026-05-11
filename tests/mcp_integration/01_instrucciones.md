# fmcp_integration — Test manual de integración MCP

## Propósito

Verificar que el modelo usa correctamente todas las herramientas MCP de Pillbox en un escenario realista. El test cubre creación, edición, soft-delete, búsqueda FTS, y comprueba que el modelo guarda capsules de forma autónoma cuando el contexto lo justifica.

## Estructura del test

| Archivo | Contenido |
|---|---|
| `02_prompt.md` | Prompt listo para copiar-pegar en sesión limpia |
| `03_esperado.md` | Checklist de verificación y comportamientos a vigilar |

## Cómo ejecutarlo

1. Abre una sesión nueva de Claude con contexto limpio (sin historial previo).
2. Asegúrate de que el servidor MCP de Pillbox está corriendo (`pillbox serve` o el MCP configurado en Claude Code).
3. Copia el contenido de `02_prompt.md` entre las marcas `--- INICIO ---` y `--- FIN ---` y pégalo en el chat.
4. Deja que el modelo ejecute todos los pasos sin interrumpirlo, salvo que cometa un error grave.
5. Al terminar, usa `03_esperado.md` para revisar el resultado con supervisión humana.

## Superficie cubierta

**Vía MCP (herramientas del modelo):**
- `bottle_create`, `bottle_context`
- `prescription_open`, `prescription_close`, `prescription_discard`, `prescription_context`
- `pill_store`, `pill_read`, `pill_revise`, `pill_discard`, `pill_search`
- `capsule_store`, `capsule_search`

**Vía WebUI (supervisión humana — sección en `03_esperado.md`):**
- Perma-delete de prescription, pill y capsule (`/purge` endpoints)

## Nota sobre perma-delete

Los endpoints de eliminación permanente (`DELETE .../purge`) no están expuestos como herramientas MCP — solo a través de la WebUI. Por eso el test los cubre en el checklist de revisión manual, no en el prompt al modelo.
