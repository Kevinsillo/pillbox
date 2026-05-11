.PHONY: dev dev-webui webui-build build build-full test lint check fmt fmt-check \
        build-linux build-mac build-win build-all \
        db-shell db-reset install

# ─── Desarrollo ───────────────────────────────────────────────────────────────

## Arranca el core Rust en modo watch (requiere cargo-watch)
dev:
	cargo watch -x 'build' --manifest-path core/Cargo.toml

## Arranca el servidor de desarrollo de la WebUI (con proxy a pillbox serve)
dev-webui:
	@pillbox serve start 2>/dev/null || true
	cd webui && pnpm dev

# ─── Build ────────────────────────────────────────────────────────────────────

## Compila la WebUI y genera dist/
webui-build:
	cd webui && pnpm install && pnpm build

## Compila la WebUI y luego el binario Rust (WebUI embebida)
build-full: webui-build
	cargo build --release --manifest-path core/Cargo.toml

## Compila el binario Rust sin WebUI (más rápido, para desarrollo del core)
build:
	cargo build --release --manifest-path core/Cargo.toml

# ─── Tests y calidad ──────────────────────────────────────────────────────────

## Ejecuta todos los tests del core Rust
test:
	cargo test --manifest-path core/Cargo.toml

## Linter Rust (warnings → errores)
lint:
	cargo clippy --manifest-path core/Cargo.toml -- -D warnings

## Formatea el código Rust
fmt:
	cargo fmt --manifest-path core/Cargo.toml

## Verifica formato sin modificar (útil en CI)
fmt-check:
	cargo fmt --manifest-path core/Cargo.toml -- --check

## Ejecuta test + lint + fmt-check
check: test lint fmt-check

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

# ─── Instalación local ───────────────────────────────────────────────────────

## Compila (webui + core) e instala el binario
install:
	@pkill -x pillbox 2>/dev/null && echo "✓ pillbox serve detenido" || true
	$(MAKE) build-full
	mkdir -p $${HOME}/.local/bin
	cp core/target/release/pillbox $${HOME}/.local/bin/pillbox
	@echo "✓ pillbox instalado en ~/.local/bin/pillbox"
	@systemctl --user start pillbox-daemon 2>/dev/null && echo "✓ pillbox serve arrancado" || true
