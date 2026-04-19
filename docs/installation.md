# Installation

## One-line install (recommended)

```bash
curl -fsSL https://get.pillbox.dev | bash
```

This script:
1. Detects your platform (Linux/macOS, x86_64/aarch64)
2. Downloads the `pillbox` binary from GitHub Releases
3. Installs it to `~/.local/bin` (Linux) or `/usr/local/bin` (macOS), or a directory of your choice
4. Installs the MCP server to `~/.pillbox/mcp/`
5. Installs the Claude Code skill to `~/.claude/skills/pillbox/`
6. Creates the global database at `~/.pillbox/pillbox.db`

### Options

**Install a specific version:**

```bash
curl -fsSL https://get.pillbox.dev | bash -s -- --version 0.2.0
```

**Custom install directory:**

```bash
PILLBOX_INSTALL_DIR=/usr/local/bin curl -fsSL https://get.pillbox.dev | bash
```

---

## Manual installation

### 1. Download the binary

Download the appropriate binary for your platform from [GitHub Releases](https://github.com/kevinsillo/pillbox/releases):

| Platform | Binary |
|---|---|
| Linux x86_64 | `pillbox-linux-x86_64` |
| Linux aarch64 | `pillbox-linux-aarch64` |
| macOS x86_64 | `pillbox-darwin-x86_64` |
| macOS arm64 | `pillbox-darwin-aarch64` |

Place it somewhere on your `$PATH` and make it executable:

```bash
chmod +x pillbox
mv pillbox ~/.local/bin/
```

### 2. Initialize the global database

```bash
pillbox --init-global
```

This creates `~/.pillbox/pillbox.db` with the full schema.

### 3. Install the MCP server

```bash
pillbox mcp install
```

This extracts the MCP server (embedded in the binary) to `~/.pillbox/mcp/`. Requires Node.js ≥ 18.

### 4. Install the Claude Code skill

```bash
pillbox skill install
```

This extracts the skill (embedded in the binary) to `~/.claude/skills/pillbox/`.

---

## Optional: port 80 and auto-start

The installer offers two optional steps:

### Port 80 (Linux)

Allows `pillbox serve` to bind to port 80 without root:

```bash
sudo setcap 'cap_net_bind_service=+ep' $(which pillbox)
```

On macOS, the installer configures a `pf` (packet filter) rule instead.

### Auto-start on boot

**Linux (systemd user service):**

```bash
mkdir -p ~/.config/systemd/user
# The installer creates ~/.config/systemd/user/pillbox.service
systemctl --user enable --now pillbox
```

**macOS (LaunchAgent):**

```bash
# The installer creates ~/Library/LaunchAgents/sh.pillbox.plist
launchctl load ~/Library/LaunchAgents/sh.pillbox.plist
```

---

## Verify installation

```bash
pillbox doctor
```

Checks binary path, global and local databases, and the active bottle for the current directory. Output language follows your system locale — override with `PILLBOX_LANG=en pillbox doctor` (see [cli.md](cli.md#language-commands)).

---

## Supported platforms

| Platform | Status |
|---|---|
| Linux x86_64 | Supported |
| Linux aarch64 | Supported |
| macOS arm64 (Apple Silicon) | Supported |
| macOS x86_64 | Supported |
| Windows | Not supported |

---

## Building from source

Requires Rust 1.75+ and Node.js 20+.

```bash
git clone https://github.com/kevinsillo/pillbox
cd pillbox

# Build WebUI + MCP + Rust binary (everything embedded)
make build-full

# Initialize global DB
./core/target/release/pillbox --init-global

# Install MCP and skill from the binary
./core/target/release/pillbox mcp install
./core/target/release/pillbox skill install
```
