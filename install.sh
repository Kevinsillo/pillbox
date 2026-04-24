#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Pillbox — Script de instalación
#
# Uso:
#   curl -fsSL https://get.pillbox.dev | bash
#   curl -fsSL https://get.pillbox.dev | bash -s -- --version 0.6.0
#   curl -fsSL https://get.pillbox.dev | bash -s -- --install-dir /usr/local/bin
#
# Variables de entorno opcionales:
#   PILLBOX_VERSION — versión a instalar (default: latest)
# =============================================================================

GITHUB_REPO="kevinsillo/pillbox"
VERSION="${PILLBOX_VERSION:-latest}"
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
    --version)     VERSION="$2"; shift 2 ;;
    --install-dir) PILLBOX_INSTALL_DIR="$2"; shift 2 ;;
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
    x86_64|amd64)  arch="x86_64"  ;;
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

  step "Instalando pillbox ${version}"
  echo -e "  ${DIM}${url}${NC}"

  download "$url" "$dest"
  chmod +x "$dest"

  if [[ "$install_dir" == "/usr/local/bin" && ! -w "/usr/local/bin" ]]; then
    sudo mv "$dest" "${install_dir}/pillbox"
  else
    mkdir -p "$install_dir"
    mv "$dest" "${install_dir}/pillbox"
  fi

  if ! command -v pillbox &>/dev/null; then
    warn "El directorio ${install_dir} no está en tu PATH."
    warn "Añade esto a tu ~/.bashrc o ~/.zshrc:"
    warn "  export PATH=\"${install_dir}:\$PATH\""
  fi

  ok "Binario instalado en ${install_dir}/pillbox"
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

  echo
  echo -e "${BOLD}  Pillbox instalado correctamente.${NC}"
  echo
  echo -e "  Ejecuta ${BOLD}pillbox --help${NC} para ver todos los comandos disponibles."
  echo -e "  Documentación completa en ${BOLD}https://pillbox.dev/docs${NC}"
  echo
}

main "$@"
