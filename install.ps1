#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir
)

# =============================================================================
# Pillbox — Script de instalación para Windows (PowerShell)
#
# Uso:
#   irm https://get.pillbox.dev/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://get.pillbox.dev/install.ps1))) -Version 0.6.0
#   .\install.ps1 -Version 0.6.0
#   .\install.ps1 -InstallDir 'C:\Tools\pillbox'
#
# Variables de entorno opcionales:
#   PILLBOX_VERSION     — versión a instalar (default: latest)
#   PILLBOX_INSTALL_DIR — directorio de instalación (default: %LOCALAPPDATA%\Programs\pillbox)
#
# Instalación a nivel de usuario (no requiere permisos de administrador).
# =============================================================================

$ErrorActionPreference = 'Stop'

$GithubRepo = 'kevinsillo/pillbox'

# Resolución de la versión: parámetro -Version > $env:PILLBOX_VERSION > 'latest'
if (-not $Version) {
    $Version = if ($env:PILLBOX_VERSION) { $env:PILLBOX_VERSION } else { 'latest' }
}

# Resolución del directorio de instalación: parámetro -InstallDir > $env:PILLBOX_INSTALL_DIR > default
if (-not $InstallDir) {
    $InstallDir = if ($env:PILLBOX_INSTALL_DIR) { $env:PILLBOX_INSTALL_DIR } else { $null }
}

# ─── Salida con formato ─────────────────────────────────────────────────────────

function Write-Step($msg) { Write-Host "`n▸ $msg" -ForegroundColor White }
function Write-Ok($msg)   { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  ⚠  $msg" -ForegroundColor Yellow }
function Write-Die($msg) {
    Write-Host "`nError: $msg" -ForegroundColor Red
    exit 1
}

# ─── Detección de arquitectura ──────────────────────────────────────────────────

function Get-Arch {
    # PROCESSOR_ARCHITECTURE refleja la arquitectura nativa de la máquina.
    $proc = $env:PROCESSOR_ARCHITEW6432
    if (-not $proc) { $proc = $env:PROCESSOR_ARCHITECTURE }

    switch ($proc) {
        'AMD64' { return 'x86_64' }
        'ARM64' { return 'aarch64' }
        default { Write-Die "Arquitectura no soportada: $proc. Solo se soportan AMD64 (x86_64) y ARM64 (aarch64)." }
    }
}

# ─── Resolución de versión ──────────────────────────────────────────────────────

function Resolve-Version {
    if ($Version -ne 'latest') {
        return $Version
    }

    $apiUrl = "https://api.github.com/repos/$GithubRepo/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ 'User-Agent' = 'pillbox-installer' }
    } catch {
        Write-Die "No se pudo consultar la API de GitHub para resolver la última versión: $($_.Exception.Message)"
    }

    $tag = $release.tag_name
    if (-not $tag) {
        Write-Die 'No se pudo determinar la última versión desde GitHub.'
    }
    return $tag
}

# ─── Directorio de instalación del binario ──────────────────────────────────────

function Get-InstallDir {
    if ($InstallDir) {
        return $InstallDir
    }
    return (Join-Path $env:LOCALAPPDATA 'Programs\pillbox')
}

# ─── Gestión del PATH de usuario (HKCU) ─────────────────────────────────────────

function Add-ToUserPath($dir) {
    # Lee el PATH de usuario directamente del registro (HKCU), no el del proceso,
    # para no arrastrar entradas heredadas de la sesión ni del PATH de sistema.
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $userPath) { $userPath = '' }

    # Idempotente: no duplicar si el directorio ya está presente (comparación case-insensitive).
    $entries = $userPath -split ';' | Where-Object { $_ -ne '' }
    $normalized = $dir.TrimEnd('\')
    foreach ($e in $entries) {
        if ($e.TrimEnd('\').Equals($normalized, [StringComparison]::OrdinalIgnoreCase)) {
            Write-Ok "El directorio ya está en el PATH de usuario."
            return
        }
    }

    # Prepende el directorio de instalación al PATH de usuario.
    $newPath = if ($userPath -eq '') { $dir } else { "$dir;$userPath" }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')

    # Refleja el cambio en el proceso actual para no obligar a reiniciar esta sesión.
    $env:Path = "$dir;$env:Path"

    Write-Ok "Directorio añadido al PATH de usuario."
    Write-Warn "Abre una nueva terminal para que el cambio de PATH surta efecto."
}

# ─── Instalación del binario ────────────────────────────────────────────────────

function Install-Binary($arch, $version, $installDir) {
    $asset = "pillbox-windows-$arch.exe"
    $url   = "https://github.com/$GithubRepo/releases/download/$version/$asset"

    Write-Step "Instalando pillbox $version"
    Write-Host "  $url" -ForegroundColor DarkGray

    # Descarga primero a una ruta temporal: si algo falla no queda un binario a
    # medias en el directorio de instalación (movimiento atómico solo en éxito).
    $tmpFile = Join-Path ([System.IO.Path]::GetTempPath()) ("pillbox-" + [System.Guid]::NewGuid().ToString() + ".exe")
    try {
        Invoke-WebRequest -Uri $url -OutFile $tmpFile -UseBasicParsing
    } catch {
        if (Test-Path $tmpFile) { Remove-Item $tmpFile -Force -ErrorAction SilentlyContinue }
        Write-Die "No se pudo descargar el binario desde $url : $($_.Exception.Message)"
    }

    # Crea el directorio de instalación si no existe.
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    $dest = Join-Path $installDir 'pillbox.exe'

    # Reinstalación idempotente: sobrescribe el binario existente.
    try {
        Move-Item -Path $tmpFile -Destination $dest -Force
    } catch {
        if (Test-Path $tmpFile) { Remove-Item $tmpFile -Force -ErrorAction SilentlyContinue }
        Write-Die "No se pudo mover el binario a $dest : $($_.Exception.Message)"
    }

    Write-Ok "Binario instalado en $dest"
    Add-ToUserPath $installDir
}

# ─── Main ───────────────────────────────────────────────────────────────────────

function Main {
    Write-Host ''
    Write-Host 'Pillbox — Instalador' -ForegroundColor White
    Write-Host "https://github.com/$GithubRepo" -ForegroundColor DarkGray

    $arch       = Get-Arch
    $version    = Resolve-Version
    $installDir = Get-InstallDir

    Write-Host "  Plataforma: windows-$arch"
    Write-Host "  Versión:    $version"
    Write-Host "  Binario:    $(Join-Path $installDir 'pillbox.exe')"

    Install-Binary $arch $version $installDir

    Write-Host ''
    Write-Host '  Pillbox instalado correctamente.' -ForegroundColor White
    Write-Host ''
    Write-Host '  Ejecuta ' -NoNewline; Write-Host 'pillbox --help' -ForegroundColor White -NoNewline; Write-Host ' para ver todos los comandos disponibles.'
    Write-Host '  Documentación completa en ' -NoNewline; Write-Host 'https://pillbox.dev/docs' -ForegroundColor White
    Write-Host ''
}

Main
