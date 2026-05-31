## [0.15.0] - 2026-05-31

### 🚀 Features

- *(update)* Añadir comando pillbox update para auto-actualización

## [0.14.3] - 2026-05-31

### 🐛 Bug Fixes

- *(install)* Invocar install.sh con bash en lugar de sh

### ⚙️ Miscellaneous Tasks

- *(release)* Cross-compilar darwin-x86_64 desde runner macos-latest ARM
- *(release)* V0.14.3

## [0.14.2] - 2026-05-31

### 🚜 Refactor

- *(install)* Descargar repos y delegar en scripts de instalación

### ⚙️ Miscellaneous Tasks

- *(release)* Optar por Node.js 24 en actions
- *(release)* V0.14.2

## [0.14.1] - 2026-05-31

### ⚡ Performance

- Reducir tamaño del binario y CSS

### ⚙️ Miscellaneous Tasks

- *(release)* Usar git-cliff --latest para notas de la versión actual
- *(release)* V0.14.1

## [0.14.0] - 2026-05-31

### 🚀 Features

- *(serve)* Pool r2d2 con spawn_blocking en handlers HTTP y LRU cap en bottle_pools
- *(mcp)* Pillbox mcp run loop NDJSON persistente con cwd-per-request
- *(search)* OR entre términos con fallback fuzzy automático
- *(mcp)* Contar pill_revise y capsule_revise como visitas
- *(webui)* Añadir tabs rendered/raw en vista de pill
- *(webui)* Copiar markdown desde la vista de detalle de pill
- *(prescription)* Permitir varias prescriptions abiertas simultáneamente
- *(core)* Eliminar BD local al borrar el último bottle
- *(dashboard)* Añadir widget de prescriptions abiertas
- *(serve)* Añadir soporte Windows
- Soportar OpenCode como segundo proveedor MCP/skill en el core
- *(uninstall)* Soporte de desinstalación en Windows e i18n de mensajes serve
- Preparar repositorio para apertura pública

### 🐛 Bug Fixes

- *(mcp)* Pill_search y pill_compounds resuelven bottle_id corto
- *(archivados)* Restaurar visibilidad de archivados en listados paginados
- *(db)* Migraciones idempotentes para arranque multi-proceso
- *(db)* Writes en bottles store y migrate usan transacción IMMEDIATE
- *(deps)* Cambiar reqwest a rustls-tls para compilación musl

### 💼 Other

- *(bottle_detail)* Añadir claves created_at y last_seen_at

### 🚜 Refactor

- *(db)* Separar esquema local y global con DbScope
- *(webui)* Alinear CapsuleCard al diseño de PrescriptionCard
- *(core)* Tracing a stderr, cwd explícito en resolve_db_path, eliminar Command::Exec
- *(serve)* Mover mod server a lib y exponer run_with_listener
- *(core)* Eliminar código muerto y variante de error sin uso
- *(core)* Unificar PRAGMA y extraer SQL inline a store
- *(webui)* Extraer helpers compartidos y poblar compounds dinámicamente
- *(output)* Split fmt.rs into responsibility modules
- *(db)* Split search.rs context API into context.rs
- *(cli)* Extract Clap enums to cli.rs and help engine to help.rs
- *(cmd/bottle)* Unify migration via db/migrate.rs and route delete/repair through output
- *(webui)* Extraer useBottleSelector y useBugReporter de MainLayout
- *(webui)* Extraer useSearch y componente SearchResultCard
- *(webui)* Mover edición de capsules a vista CapsuleEditView
- *(cmd/bottle)* Enrutar eprintln de init y vinculate por output::bottle

### 📚 Documentation

- *(readme)* Añadir gif de demo del CLI
- Añadir captura del dashboard WebUI al README
- Reemplazar localhost:4242 por pillbox.local:4242 en README

### 🎨 Styling

- *(mcp)* Aplicar cargo fmt a pills.rs

### 🧪 Testing

- *(mcp)* Hacer el prompt de integración agnóstico al proyecto

### ⚙️ Miscellaneous Tasks

- *(install)* Traducir mensajes de los instaladores a inglés
- *(webui)* Eliminar Pinia sin uso y componentes muertos
- Corregir lints de clippy 1.94 en tests de integración
- *(release)* V0.14.0
- *(release)* Usar Node.js 22 requerido por pnpm 11
- *(release)* Publicar binarios de forma asíncrona según terminan

## [0.13.0] - 2026-05-17

### 🚀 Features

- *(webui)* Menú de prompts para agente en bottle/rx/pill/capsule
- *(views)* Contador de vistas por lectura MCP en bottles/prescriptions/pills/capsules
- *(webui)* Añadir widget de tamaño de la base de datos en el dashboard
- *(webui)* Botón en sidebar para reportar bugs vía issue form de GitHub

### 🐛 Bug Fixes

- *(webui)* Scroll horizontal en tablas del markdown
- *(mcp)* Pill_search y capsule_search aceptan compound sin query

### 🎨 Styling

- *(webui)* Quitar transition-colors en componentes y vistas

### ⚙️ Miscellaneous Tasks

- *(release)* V0.13.0

## [0.12.0] - 2026-05-16

### 🚀 Features

- Merge compounds-from-json → main (v0.11.0)
- Añadir búsqueda por prefijo de ID al estilo git en stores, HTTP y WebUI
- Reabrir prescriptions y bloquear edición de pills en rx cerradas
- *(search)* Búsqueda configurable con fuzzy opcional y compounds discoverable
- *(pagination)* Paginación reutilizable en listados (HTTP + WebUI)

### 🐛 Bug Fixes

- *(handlers)* Resolver UUID completo antes de usar como foreign key
- *(ids)* Ampliar ID corto a 12 hex chars sin guiones
- *(ids)* Ampliar shortId a 12 hex chars sin guiones en webui
- *(ids)* Corregir resolución de prefijos en registered_bottles y search
- *(search)* Deserialización de query string con campos flatten

### 💼 Other

- Traducir claves CLI reopen en de/fr/it/pt

### 🚜 Refactor

- *(mcp)* Reemplazar ContextResult formateado por structs estructurados
- *(mcp)* Unificar errores tipados y mensajes en inglés por defecto

### 🧪 Testing

- *(ids)* Añadir cobertura de resolución por prefijo de 12 chars

### ⚙️ Miscellaneous Tasks

- *(release)* V0.12.0

## [0.11.0] - 2026-05-11

### 🚀 Features

- *(webui)* Añadir TruncatedTitle con tooltip en tarjetas y vistas
- *(mcp)* Reemplazar pill_context por bottle_context y prescription_context
- Renombrar compounds pattern→specification y manual→task en webui
- Añadir roadmap visual al README
- *(readme)* Reemplazar logos PNG por SVG adaptativo claro/oscuro y añadir banner de estrella GitHub
- *(readme)* Añadir roadmap SVG adaptativo y rediseñar banner de estrella GitHub
- *(webui)* Añadir syntax highlighting y mejorar estilos markdown
- *(cli)* Limitar registros archivados en listados con trailer

### 🐛 Bug Fixes

- *(webui)* Desacoplar endpoint /context de las MCP tools
- *(readme)* Corregir licencia — PolyForm Noncommercial, no MIT
- *(roadmap)* Cambiar etiqueta MIT release por Public release
- *(serve)* Activar autostart al instalar el servicio del sistema
- *(webui)* Escapar HTML en bloques de código sin lenguaje reconocido

### 💼 Other

- *(webui)* Mejoras visuales en checkboxes, código y cards
- *(webui)* Checkboxes markdown con colores del design system

### 🚜 Refactor

- *(mcp)* Aplanar parámetros de revise para pills y capsules
- Eliminar tablas dispense_log y action_types
- Renombrar compound prescription_summary a summary
- *(compounds)* Eliminar enums y usar string libre como compound
- *(db)* Migrar pills y capsules a UUID v7 como clave primaria

### 🧪 Testing

- Añadir test manual de integración MCP (fmcp_integration)

### ⚙️ Miscellaneous Tasks

- *(release)* V0.11.0

## [0.10.0] - 2026-05-03

### 🚀 Features

- *(core)* Añadir author_name/author_email a prescriptions en todas las capas
- *(author)* Mostrar autor en cli, webui y mcp
- *(cli)* Añadir límite paginado a todos los comandos list y prescription show
- *(bottle)* Añadir subcomando vinculate para registro multi-usuario
- *(serve)* Reemplazar daemon inline por servicio del sistema OS
- *(webui)* Añadir polling en tiempo real y animaciones de lista

### 🐛 Bug Fixes

- Ajustes en CLI, MCP y webui
- *(migrate)* Corregir actualización de scope y db_path al migrar bottle
- *(webui)* Corregir colores del chip "archivada" en modo claro y añadirlo a prescriptions
- *(webui)* Corregir chip archivada en prescription cards de BottleDetailView

### 🚜 Refactor

- *(webui)* Separar botón de borrar en archivar y eliminar definitivamente
- *(core)* Eliminar código muerto no utilizado
- *(uninstall)* Confirmación única con lista de componentes a eliminar

### 📚 Documentation

- *(core)* Añadir documentación rustdoc a todos los módulos y funciones públicas

### 🎨 Styling

- *(webui)* Añadir icono de clipboard antes del nombre en detalle de prescription
- *(output)* Mejorar formato de lang show y tabla dict

### ⚙️ Miscellaneous Tasks

- *(release)* V0.10.0

## [0.9.0] - 2026-04-30

### 🚀 Features

- *(mcp)* Incluir title, compound y content en PillTakeResult y CapsuleTakeResult

### 🐛 Bug Fixes

- [**breaking**] Eliminar campo dispenser de pills
- *(webui)* Corregir título de la ventana del dashboard
- *(mcp)* Corregir bottle_create para crear en la DB correcta según scope

### 🚜 Refactor

- *(mcp)* Renombrar pill_take → pill_store y capsule_take → capsule_store

### ⚙️ Miscellaneous Tasks

- *(release)* V0.9.0

## [0.8.0] - 2026-04-28

### 🚀 Features

- Modal de doble acción para borrado de prescriptions y pills
- *(archivados)* Visibilidad de soft deletes y hard delete en cápsulas

### 🐛 Bug Fixes

- *(mcp)* Registrar bottle en global registry al crearlo vía tool
- *(core)* Completar cascade y limpieza al eliminar bottle

### ⚙️ Miscellaneous Tasks

- Añadir ficheros de licencia
- Añadir identificador SPDX a la licencia
- *(release)* V0.8.0

## [0.7.0] - 2026-04-24

### 🚀 Features

- *(cli)* Añadir comandos bottle delete y bottle repair
- *(server)* Añadir endpoint GET /bottles/:id/stats
- *(webui)* Añadir PillsActivityChart y estadísticas en dashboard
- *(webui)* ConfirmDialog personalizado y migrar ElMessageBox

### 🐛 Bug Fixes

- *(webui)* Resaltar nombres en confirmaciones de borrado

### 🎨 Styling

- *(webui)* Rediseñar indicadores de estado en bottles y prescriptions

### ⚙️ Miscellaneous Tasks

- Añadir cliff.toml con salto de línea entre releases
- Regenerar CHANGELOG.md con saltos de línea entre releases
- *(core)* Traducir mensajes de error internos a inglés
- Actualizar install.sh — pasar --install-dir como argumento
- *(core)* Formatear error.rs
- *(release)* V0.7.0

## [0.6.0] - 2026-04-22

### 🚀 Features

- *(webui)* Añadir tema claro/oscuro y mejorar consistencia visual

### ⚙️ Miscellaneous Tasks

- *(release)* V0.6.0

## [0.5.0] - 2026-04-22

### 🚀 Features

- *(cli)* Rediseñar list/detail — bottle list, pill show, capsule show, prescription show

### 📚 Documentation

- *(readme)* Eliminar secciones Built with e History (movidas a la documentación)
- *(readme)* Simplificar CLI reference y apuntar a la documentación

### ⚙️ Miscellaneous Tasks

- *(release)* V0.5.0

## [0.4.1] - 2026-04-21

### 🐛 Bug Fixes

- Corregir logos invertidos en modo claro y oscuro
- Renombrar pillbox-logo.png a pillbox-logo-light.png

### 🚜 Refactor

- *(cli,webui,server)* Unificar output CLI, mejorar errores y i18n
- *(core,webui)* Migrar bottles.id a UUID y rutas REST anidadas

### 📚 Documentation

- Mencionar soporte multilenguaje en README

### 🧪 Testing

- *(core)* Ampliar cobertura de 40 a 99 tests en 11 módulos

### ⚙️ Miscellaneous Tasks

- Reorganizar estructura multirepo y actualizar assets
- *(release)* V0.4.1

## [0.4.0] - 2026-04-19

### 🚀 Features

- Implementa fase 1 — estructura core Rust y capa de DB
- *(core)* Implementar stores CRUD + búsqueda FTS5 (fase 2)
- *(core)* Añadir dispatcher exec y servidor HTTP Axum (fase 3)
- *(mcp)* Implementar servidor MCP TypeScript (fase 4)
- Skill Pillbox, Makefile completo y bottle migrate (fase 5)
- *(cli)* Implementar CLI completo — fase 6a
- Añadir install.sh multiplataforma
- *(server)* Publicar servicio mDNS al arrancar pillbox serve
- *(install)* Añadir configuración opcional de puerto 80
- *(install)* Añadir auto-start al arranque
- *(output)* Añadir módulo output con tabled y owo-colors
- *(cli)* Normalizar ayuda y salidas visuales en recuadros
- *(webui)* Añadir embedding rust-embed y completar API REST
- *(i18n)* Añadir internacionalización al CLI con rust-i18n v4 + sys-locale
- *(webui)* Añadir internacionalización (vue-i18n) e iconos Lucide en sidebar
- Completar WebUI, mejorar CLI y actualizar docs
- *(search)* Añadir búsqueda por prefijo y fuzzy con rayon + strsim
- *(core)* Completar i18n CLI en 6 idiomas
- *(core)* Mejoras en servidor HTTP y assets embebidos
- *(webui)* Refactorizar API layer a arquitectura hexagonal
- *(webui)* Completar i18n en 6 idiomas
- *(webui)* Actualizar vistas, componentes y estilos
- *(mcp)* Ampliar herramientas y esquemas
- *(core)* Implementar global bottle registry
- *(error)* Capturar ConstraintViolation como BottleAlreadyExists

### 🐛 Bug Fixes

- *(db)* Filtrar PRAGMAs en apply() para evitar error en transacción
- *(core)* Corregir detección de home dir como DB local, saltos de línea y GET /

### 💼 Other

- Actualizar Makefile y skills del proyecto

### 🚜 Refactor

- *(core)* Mejorar arquitectura del core Rust (fases 1-3)
- *(core)* Reemplazar assets embebidos por descarga desde GitHub
- *(install)* Delegar instalación de mcp y skill a manifest externo
- *(mcp)* Extraer exec.rs a módulo mcp/ con handlers por dominio
- *(server)* Dividir handlers.rs en módulo handlers/ por dominio
- *(error)* PillboxError.code() + anyhow_to_response tipado

### 📚 Documentation

- *(fase-7)* Añadir documentación pública y dominio definitivo
- *(backlog)* Marcar v0.2.0 como completado y registrar mejoras realizadas
- Actualizar documentación completa

### ⚙️ Miscellaneous Tasks

- Commit inicial con documentación de diseño y skills de Claude
- Añadir autoformato con cargo fmt y prettier
- *(webui)* Añadir scaffold Vue + lockfile MCP
- *(release)* V0.4.0

