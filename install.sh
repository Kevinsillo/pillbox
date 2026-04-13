#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Pillbox — Script de instalación
#
# Uso:
#   curl -fsSL https://get.pillbox.sh | bash
#   curl -fsSL https://get.pillbox.sh | bash -s -- --version 0.2.0
#
# Variables de entorno opcionales:
#   PILLBOX_VERSION     — versión a instalar (default: latest)
#   PILLBOX_INSTALL_DIR — directorio del binario (default: auto)
# =============================================================================

# -----------------------------------------------------------------------------
# TODO: actualizar cuando el repo público esté creado
GITHUB_REPO="OWNER/pillbox-releases"
# -----------------------------------------------------------------------------

VERSION="${PILLBOX_VERSION:-latest}"
MCP_DIR="${HOME}/.pillbox/mcp"
SKILL_DIR="${HOME}/.claude/skills/pillbox"
TMPDIR_WORK="$(mktemp -d)"

# ─── Colores ──────────────────────────────────────────────────────────────────

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
DIM='\033[2m'
NC='\033[0m'

step()  { echo -e "\n${BOLD}▸ $*${NC}"; }
ok()    { echo -e "  ${GREEN}✓${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC}  $*"; }
die()   { echo -e "\n${RED}Error:${NC} $*" >&2; exit 1; }

# ─── Limpieza al salir ────────────────────────────────────────────────────────

cleanup() { rm -rf "$TMPDIR_WORK"; }
trap cleanup EXIT

# ─── Argumentos ───────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    *) die "Argumento desconocido: $1" ;;
  esac
done

# ─── Detección de plataforma ──────────────────────────────────────────────────

detect_platform() {
  local os arch

  case "$(uname -s)" in
    Linux)  os="linux"  ;;
    Darwin) os="darwin" ;;
    *)      die "Sistema operativo no soportado: $(uname -s). Usa Linux o macOS." ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64"  ;;
    aarch64|arm64) arch="aarch64" ;;
    *) die "Arquitectura no soportada: $(uname -m)." ;;
  esac

  echo "${os}-${arch}"
}

# ─── Descargador (curl o wget) ────────────────────────────────────────────────

download() {
  local url="$1" dest="$2"
  if command -v curl &>/dev/null; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget &>/dev/null; then
    wget -qO "$dest" "$url"
  else
    die "Se necesita curl o wget para instalar Pillbox."
  fi
}

# ─── Resolución de versión ────────────────────────────────────────────────────

resolve_version() {
  if [[ "$VERSION" != "latest" ]]; then
    echo "$VERSION"
    return
  fi

  local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
  local tmpfile="${TMPDIR_WORK}/release.json"
  download "$api_url" "$tmpfile"

  local tag
  tag="$(grep '"tag_name"' "$tmpfile" | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
  [[ -n "$tag" ]] || die "No se pudo determinar la última versión desde GitHub."
  echo "$tag"
}

# ─── Directorio de instalación del binario ────────────────────────────────────

find_install_dir() {
  if [[ -n "${PILLBOX_INSTALL_DIR:-}" ]]; then
    echo "$PILLBOX_INSTALL_DIR"
    return
  fi
  # Preferir /usr/local/bin si es escribible, si no ~/.local/bin
  if [[ -w "/usr/local/bin" ]]; then
    echo "/usr/local/bin"
  elif sudo -n true 2>/dev/null; then
    echo "/usr/local/bin"
  else
    echo "${HOME}/.local/bin"
  fi
}

# ─── Instalación del binario ──────────────────────────────────────────────────

install_binary() {
  local platform="$1" version="$2" install_dir="$3"
  local asset="pillbox-${platform}"
  local url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${asset}"
  local dest="${TMPDIR_WORK}/pillbox"

  step "Instalando binario pillbox ${version}"
  echo -e "  ${DIM}${url}${NC}"

  download "$url" "$dest"
  chmod +x "$dest"

  if [[ "$install_dir" == "/usr/local/bin" && ! -w "/usr/local/bin" ]]; then
    sudo mv "$dest" "${install_dir}/pillbox"
  else
    mkdir -p "$install_dir"
    mv "$dest" "${install_dir}/pillbox"
  fi

  # Asegurar que el directorio está en PATH
  if ! command -v pillbox &>/dev/null; then
    warn "El directorio ${install_dir} no está en tu PATH."
    warn "Añade esto a tu ~/.bashrc o ~/.zshrc:"
    warn "  export PATH=\"${install_dir}:\$PATH\""
  fi

  ok "Binario instalado en ${install_dir}/pillbox"
}

# ─── Instalación del servidor MCP ─────────────────────────────────────────────

install_mcp() {
  local version="$1"
  local url="https://github.com/${GITHUB_REPO}/releases/download/${version}/pillbox-mcp.tar.gz"
  local tarball="${TMPDIR_WORK}/pillbox-mcp.tar.gz"

  step "Instalando servidor MCP"

  if ! command -v node &>/dev/null; then
    warn "Node.js no está instalado. El servidor MCP requiere Node.js ≥ 18."
    warn "Instálalo desde https://nodejs.org y luego ejecuta:"
    warn "  curl -fsSL https://get.pillbox.sh | bash"
    return
  fi

  local node_version
  node_version="$(node --version | sed 's/v//' | cut -d. -f1)"
  if [[ "$node_version" -lt 18 ]]; then
    warn "Node.js ${node_version} detectado. Pillbox MCP requiere Node.js ≥ 18."
    return
  fi

  download "$url" "$tarball"
  mkdir -p "$MCP_DIR"
  tar -xzf "$tarball" -C "$MCP_DIR" --strip-components=1

  ok "MCP instalado en ${MCP_DIR}/index.js"
}

# ─── Instalación de la skill ──────────────────────────────────────────────────

install_skill() {
  local version="$1"
  local url="https://github.com/${GITHUB_REPO}/releases/download/${version}/SKILL.md"

  step "Instalando skill de Claude Code"
  mkdir -p "$SKILL_DIR"
  download "$url" "${SKILL_DIR}/SKILL.md"
  ok "Skill instalada en ${SKILL_DIR}/SKILL.md"
}

# ─── Inicializar DB global ────────────────────────────────────────────────────

init_db() {
  step "Inicializando DB global"

  if pillbox --init-global; then
    ok "DB global creada en ~/.pillbox/pillbox.db"
  else
    warn "No se pudo inicializar la DB global. Ejecuta 'pillbox --init-global' manualmente."
  fi
}

# ─── Privilegios de puerto 80 ────────────────────────────────────────────────

setup_port_80() {
  local os="$1" install_dir="$2"

  step "Puerto 80 (opcional)"
  echo -e "  Permite usar ${BOLD}pillbox serve --port 80${NC} y acceder como ${BOLD}http://pillbox.local${NC}."
  echo -e -n "  ¿Configurar ahora? [s/N] "
  read -r answer
  [[ ! "$answer" =~ ^[Ss] ]] && return

  case "$os" in
    linux)  _port80_linux  "$install_dir" ;;
    darwin) _port80_macos ;;
  esac
}

_port80_linux() {
  local binary="${1}/pillbox"

  if ! command -v setcap &>/dev/null; then
    warn "setcap no encontrado. Instala libcap2-bin y vuelve a ejecutar:"
    warn "  sudo apt install libcap2-bin"
    warn "  sudo setcap cap_net_bind_service=+ep ${binary}"
    return
  fi

  if sudo setcap cap_net_bind_service=+ep "$binary" 2>/dev/null; then
    ok "Capability cap_net_bind_service asignada al binario."
    ok "Inicia el servidor con: pillbox serve --port 80"
  else
    warn "No se pudo ejecutar setcap. Intenta manualmente:"
    warn "  sudo setcap cap_net_bind_service=+ep ${binary}"
  fi
}

_port80_macos() {
  local anchor_file="/etc/pf.anchors/pillbox"
  local pf_conf="/etc/pf.conf"
  local default_port="${PILLBOX_DEFAULT_PORT:-4242}"

  # Crear el anchor de pf con la regla de redirección
  sudo tee "$anchor_file" > /dev/null <<EOF
# Pillbox — redirige el puerto 80 al servidor local en ${default_port}
rdr pass on lo0 inet proto tcp from any to 127.0.0.1 port 80 -> 127.0.0.1 port ${default_port}
rdr pass on en0 inet proto tcp from any to any       port 80 -> 127.0.0.1 port ${default_port}
EOF

  # Añadir referencia al anchor en pf.conf si no existe
  if ! sudo grep -q 'anchor "pillbox"' "$pf_conf" 2>/dev/null; then
    sudo tee -a "$pf_conf" > /dev/null <<'EOF'

# Pillbox — redirección de puerto 80
rdr-anchor "pillbox"
anchor "pillbox"
load anchor "pillbox" from "/etc/pf.anchors/pillbox"
EOF
  fi

  # Activar pf y cargar las reglas
  if sudo pfctl -f "$pf_conf" -e 2>/dev/null; then
    ok "pf activado con regla de redirección 80 → ${default_port}."
    ok "El puerto 80 volverá al estado anterior al reiniciar."
    ok "Usa 'pillbox autostart' para hacerlo permanente (paso siguiente)."
  else
    warn "No se pudo cargar pf. Actívalo manualmente:"
    warn "  sudo pfctl -f ${pf_conf} -e"
  fi
}

# ─── Auto-start al arranque ──────────────────────────────────────────────────

setup_autostart() {
  local os="$1" install_dir="$2"

  step "Auto-start al arranque (opcional)"
  echo -e "  Inicia ${BOLD}pillbox serve${NC} automáticamente al encender el equipo."
  echo -e -n "  ¿Configurar ahora? [s/N] "
  read -r answer
  [[ ! "$answer" =~ ^[Ss] ]] && return

  case "$os" in
    linux)  _autostart_linux  "$install_dir" ;;
    darwin) _autostart_macos  "$install_dir" ;;
  esac
}

_autostart_linux() {
  local binary="${1}/pillbox"
  local unit_dir="${HOME}/.config/systemd/user"
  local unit_file="${unit_dir}/pillbox.service"

  mkdir -p "$unit_dir"

  cat > "$unit_file" <<EOF
[Unit]
Description=Pillbox — Persistent memory server
After=default.target

[Service]
Type=simple
ExecStart=${binary} serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

  if systemctl --user enable pillbox.service 2>/dev/null &&
     systemctl --user start  pillbox.service 2>/dev/null; then
    ok "Servicio systemd instalado y arrancado."
    ok "Estado: systemctl --user status pillbox"
  else
    warn "No se pudo habilitar el servicio systemd."
    warn "Actívalo manualmente:"
    warn "  systemctl --user enable pillbox && systemctl --user start pillbox"
  fi

  # Sin lingering, el servicio no arranca si el usuario no tiene sesión activa
  if ! loginctl show-user "$USER" 2>/dev/null | grep -q "Linger=yes"; then
    warn "Para que arranque sin sesión abierta, habilita linger:"
    warn "  loginctl enable-linger ${USER}"
  fi
}

_autostart_macos() {
  local binary="${1}/pillbox"
  local plist_dir="${HOME}/Library/LaunchAgents"
  local plist_file="${plist_dir}/sh.pillbox.server.plist"
  local log_file="${HOME}/.pillbox/pillbox-serve.log"

  mkdir -p "$plist_dir"

  cat > "$plist_file" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>sh.pillbox.server</string>

  <key>ProgramArguments</key>
  <array>
    <string>${binary}</string>
    <string>serve</string>
  </array>

  <key>RunAtLoad</key>
  <true/>

  <key>KeepAlive</key>
  <true/>

  <key>StandardOutPath</key>
  <string>${log_file}</string>

  <key>StandardErrorPath</key>
  <string>${log_file}</string>
</dict>
</plist>
EOF

  # launchctl bootstrap (macOS ≥ 10.15) con fallback a load (versiones antiguas)
  local plist_domain="gui/$(id -u)"
  if launchctl bootstrap "$plist_domain" "$plist_file" 2>/dev/null; then
    ok "LaunchAgent instalado y arrancado."
  elif launchctl load -w "$plist_file" 2>/dev/null; then
    ok "LaunchAgent instalado y arrancado (modo compatibilidad)."
  else
    warn "No se pudo cargar el LaunchAgent."
    warn "Cárgalo manualmente:"
    warn "  launchctl load -w ${plist_file}"
  fi

  ok "Logs en: ${log_file}"
  ok "Estado:  launchctl list sh.pillbox.server"
}

# ─── Instrucciones de configuración del MCP ───────────────────────────────────

print_mcp_config() {
  local mcp_path="${MCP_DIR}/index.js"

  echo
  echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
  echo -e "${BOLD}  Pillbox instalado correctamente.${NC}"
  echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
  echo
  echo -e "Para activar Pillbox en Claude Code, añade esto a tu"
  echo -e "configuración MCP (~/.claude.json o claude_desktop_config.json):"
  echo
  echo -e "${DIM}{${NC}"
  echo -e "${DIM}  \"mcpServers\": {${NC}"
  echo -e "${DIM}    \"pillbox\": {${NC}"
  echo -e "${DIM}      \"command\": \"node\",${NC}"
  echo -e "${DIM}      \"args\": [\"${mcp_path}\"]${NC}"
  echo -e "${DIM}    }${NC}"
  echo -e "${DIM}  }${NC}"
  echo -e "${DIM}}${NC}"
  echo
  echo -e "Después reinicia Claude Code y verifica con:"
  echo -e "  ${BOLD}pillbox doctor${NC}"
  echo
}

# ─── Main ─────────────────────────────────────────────────────────────────────

main() {
  echo
  echo -e "${BOLD}Pillbox — Instalador${NC}"
  echo -e "${DIM}https://github.com/${GITHUB_REPO}${NC}"

  local platform version install_dir
  platform="$(detect_platform)"
  version="$(resolve_version)"
  install_dir="$(find_install_dir)"

  echo -e "  Plataforma: ${platform}"
  echo -e "  Versión:    ${version}"
  echo -e "  Binario:    ${install_dir}/pillbox"

  install_binary "$platform" "$version" "$install_dir"
  install_mcp    "$version"
  install_skill  "$version"
  init_db
  setup_port_80   "${platform%%-*}" "$install_dir"
  setup_autostart "${platform%%-*}" "$install_dir"
  print_mcp_config
}

main "$@"
