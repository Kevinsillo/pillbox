.PHONY: dev build test lint build-linux build-mac build-win build-all migrate db-shell db-reset mcp-install mcp-dev

# Desarrollo
dev:
	cargo watch -x run --manifest-path core/Cargo.toml

build:
	cargo build --release --manifest-path core/Cargo.toml

test:
	cargo test --manifest-path core/Cargo.toml

lint:
	cargo clippy --manifest-path core/Cargo.toml -- -D warnings

# Distribución
build-linux:
	cargo build --release --target x86_64-unknown-linux-musl --manifest-path core/Cargo.toml

build-mac:
	cargo build --release --target aarch64-apple-darwin --manifest-path core/Cargo.toml
	cargo build --release --target x86_64-apple-darwin --manifest-path core/Cargo.toml
	lipo -create -output target/pillbox-mac \
		core/target/aarch64-apple-darwin/release/pillbox \
		core/target/x86_64-apple-darwin/release/pillbox

build-win:
	cargo build --release --target x86_64-pc-windows-gnu --manifest-path core/Cargo.toml

build-all: build-linux build-mac build-win

# DB
migrate:
	./target/release/pillbox migrate

db-shell:
	sqlite3 $${PILLBOX_DB:-$$HOME/.pillbox/pillbox.db}

db-reset:
	rm -f .pillbox/pillbox.db $$HOME/.pillbox/pillbox.db
	@echo "DB eliminada. Ejecuta 'pillbox init' para recrear."

# MCP + Skill (desarrollo — instala desde source local)
mcp-install: build
	cd mcp && npm install && npm run build
	cp -r skill ~/.claude/skills/pillbox

mcp-dev:
	cd mcp && PILLBOX_BIN=../core/target/debug/pillbox npm run dev
