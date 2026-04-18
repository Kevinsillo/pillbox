.PHONY: dev dev-mcp dev-webui webui-build build build-full test lint check fmt fmt-check \
        build-linux build-mac build-win build-all \
        db-shell db-reset \
        mcp-build mcp-install mcp-dev \
        skill-install

# ─── Desarrollo ───────────────────────────────────────────────────────────────

## Arranca el core Rust en modo watch (requiere cargo-watch)
dev:
	cargo watch -x 'build' --manifest-path core/Cargo.toml

## Arranca el servidor MCP en modo watch (apunta al binario de debug)
dev-mcp: mcp-build
	cd mcp && PILLBOX_BIN=../core/target/debug/pillbox node --watch dist/index.js

## Arranca el servidor de desarrollo de la WebUI (con proxy a pillbox serve)
dev-webui:
	cd webui && npm run dev

# ─── Build ────────────────────────────────────────────────────────────────────

## Compila la WebUI y genera dist/
webui-build:
	cd webui && npm run build

## Compila la WebUI y luego el binario Rust (webui embebida)
build-full: webui-build
	cargo build --release --manifest-path core/Cargo.toml

## Compila el binario Rust sin WebUI (más rápido, para desarrollo del core)
build:
	cargo build --release --manifest-path core/Cargo.toml

## Compila el servidor MCP TypeScript
mcp-build:
	cd mcp && npm install && npm run build

# ─── Tests y calidad ──────────────────────────────────────────────────────────

## Ejecuta todos los tests del core Rust
test:
	cargo test --manifest-path core/Cargo.toml

## Linter Rust (warnings → errores)
lint:
	cargo clippy --manifest-path core/Cargo.toml -- -D warnings

## Verifica tipos del MCP TypeScript sin compilar
typecheck:
	cd mcp && npm run typecheck

## Formatea todo el código: Rust (cargo fmt) + TypeScript (prettier)
fmt:
	cargo fmt --manifest-path core/Cargo.toml
	cd mcp && npm run fmt

## Verifica formato sin modificar (útil en CI)
fmt-check:
	cargo fmt --manifest-path core/Cargo.toml -- --check
	cd mcp && npm run fmt:check

## Ejecuta test + lint + typecheck + fmt-check
check: test lint typecheck fmt-check

# ─── Distribución ─────────────────────────────────────────────────────────────

build-linux:
	cargo build --release --target x86_64-unknown-linux-musl --manifest-path core/Cargo.toml

build-mac:
	cargo build --release --target aarch64-apple-darwin --manifest-path core/Cargo.toml
	cargo build --release --target x86_64-apple-darwin --manifest-path core/Cargo.toml
	lipo -create -output core/target/pillbox-mac \
		core/target/aarch64-apple-darwin/release/pillbox \
		core/target/x86_64-apple-darwin/release/pillbox

build-win:
	cargo build --release --target x86_64-pc-windows-gnu --manifest-path core/Cargo.toml

build-all: build-linux build-mac build-win

# ─── DB ───────────────────────────────────────────────────────────────────────

## Abre la DB activa en sqlite3
db-shell:
	sqlite3 $$([ -f .pillbox/pillbox.db ] && echo .pillbox/pillbox.db || echo $${HOME}/.pillbox/pillbox.db)

## Elimina la DB local y global (desarrollo)
db-reset:
	rm -f .pillbox/pillbox.db $${HOME}/.pillbox/pillbox.db
	@echo "DBs eliminadas."

# ─── Instalación (desarrollo local) ──────────────────────────────────────────

## Instala el MCP y la skill desde el source local
mcp-install: build mcp-build skill-install
	@echo "Pillbox MCP instalado."
	@echo "Añade esto a tu configuración de Claude Code:"
	@echo '  { "mcpServers": { "pillbox": { "command": "node", "args": ["$(CURDIR)/mcp/dist/index.js"] } } }'

## Copia la skill al directorio de skills de Claude Code
skill-install:
	mkdir -p $${HOME}/.claude/skills/pillbox
	cp -r skill/* $${HOME}/.claude/skills/pillbox/
	@echo "Skill instalada en ~/.claude/skills/pillbox/"
