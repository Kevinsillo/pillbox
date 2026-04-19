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
- *(roadmap)* Marcar fase 6 como completada
