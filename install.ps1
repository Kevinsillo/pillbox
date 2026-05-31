#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir
)

# =============================================================================
# Pillbox — Windows installation script (PowerShell)
#
# Usage:
#   irm https://raw.githubusercontent.com/Kevinsillo/pillbox/main/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/Kevinsillo/pillbox/main/install.ps1))) -Version 0.15.3
#   .\install.ps1 -Version 0.6.0
#   .\install.ps1 -InstallDir 'C:\Tools\pillbox'
#
# Optional environment variables:
#   PILLBOX_VERSION     — version to install (default: latest)
#   PILLBOX_INSTALL_DIR — installation directory (default: %LOCALAPPDATA%\Programs\pillbox)
#
# User-level installation (no administrator privileges required).
# =============================================================================

$ErrorActionPreference = 'Stop'

$GithubRepo = 'kevinsillo/pillbox'

# Version resolution: -Version parameter > $env:PILLBOX_VERSION > 'latest'
if (-not $Version) {
    $Version = if ($env:PILLBOX_VERSION) { $env:PILLBOX_VERSION } else { 'latest' }
}

# Install dir resolution: -InstallDir parameter > $env:PILLBOX_INSTALL_DIR > default
if (-not $InstallDir) {
    $InstallDir = if ($env:PILLBOX_INSTALL_DIR) { $env:PILLBOX_INSTALL_DIR } else { $null }
}

# ─── Formatted output ─────────────────────────────────────────────────────────

function Write-Step($msg) { Write-Host "`n▸ $msg" -ForegroundColor White }
function Write-Ok($msg)   { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  ⚠  $msg" -ForegroundColor Yellow }
function Write-Die($msg) {
    Write-Host "`nError: $msg" -ForegroundColor Red
    exit 1
}

# ─── Architecture detection ─────────────────────────────────────────────────────

function Get-Arch {
    # PROCESSOR_ARCHITECTURE reflects the machine's native architecture.
    $proc = $env:PROCESSOR_ARCHITEW6432
    if (-not $proc) { $proc = $env:PROCESSOR_ARCHITECTURE }

    switch ($proc) {
        'AMD64' { return 'x86_64' }
        'ARM64' { return 'aarch64' }
        default { Write-Die "Unsupported architecture: $proc. Only AMD64 (x86_64) and ARM64 (aarch64) are supported." }
    }
}

# ─── Version resolution ──────────────────────────────────────────────────────────

function Resolve-Version {
    if ($Version -ne 'latest') {
        return $Version
    }

    $apiUrl = "https://api.github.com/repos/$GithubRepo/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ 'User-Agent' = 'pillbox-installer' }
    } catch {
        Write-Die "Could not query the GitHub API to resolve the latest version: $($_.Exception.Message)"
    }

    $tag = $release.tag_name
    if (-not $tag) {
        Write-Die 'Could not determine the latest version from GitHub.'
    }
    return $tag
}

# ─── Binary install directory ──────────────────────────────────────────────────

function Get-InstallDir {
    if ($InstallDir) {
        return $InstallDir
    }
    return (Join-Path $env:LOCALAPPDATA 'Programs\pillbox')
}

# ─── User PATH management (HKCU) ────────────────────────────────────────────────

function Add-ToUserPath($dir) {
    # Read the user PATH directly from the registry (HKCU), not the process one,
    # to avoid carrying over inherited session entries or the system PATH.
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $userPath) { $userPath = '' }

    # Idempotent: do not duplicate if the directory is already present (case-insensitive).
    $entries = $userPath -split ';' | Where-Object { $_ -ne '' }
    $normalized = $dir.TrimEnd('\')
    foreach ($e in $entries) {
        if ($e.TrimEnd('\').Equals($normalized, [StringComparison]::OrdinalIgnoreCase)) {
            Write-Ok "Directory is already on the user PATH."
            return
        }
    }

    # Prepend the install directory to the user PATH.
    $newPath = if ($userPath -eq '') { $dir } else { "$dir;$userPath" }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')

    # Reflect the change in the current process so this session need not restart.
    $env:Path = "$dir;$env:Path"

    Write-Ok "Directory added to the user PATH."
    Write-Warn "Open a new terminal for the PATH change to take effect."
}

# ─── Binary installation ────────────────────────────────────────────────────────

function Install-Binary($arch, $version, $installDir) {
    $asset = "pillbox-windows-$arch.exe"
    $url   = "https://github.com/$GithubRepo/releases/download/$version/$asset"

    Write-Step "Installing pillbox $version"
    Write-Host "  $url" -ForegroundColor DarkGray

    # Download to a temporary path first: if anything fails, no half-written binary
    # is left in the install directory (atomic move only on success).
    $tmpFile = Join-Path ([System.IO.Path]::GetTempPath()) ("pillbox-" + [System.Guid]::NewGuid().ToString() + ".exe")
    try {
        Invoke-WebRequest -Uri $url -OutFile $tmpFile -UseBasicParsing
    } catch {
        if (Test-Path $tmpFile) { Remove-Item $tmpFile -Force -ErrorAction SilentlyContinue }
        Write-Die "Could not download the binary from $url : $($_.Exception.Message)"
    }

    # Create the install directory if it does not exist.
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    $dest = Join-Path $installDir 'pillbox.exe'

    # Idempotent reinstall: overwrite the existing binary.
    try {
        Move-Item -Path $tmpFile -Destination $dest -Force
    } catch {
        if (Test-Path $tmpFile) { Remove-Item $tmpFile -Force -ErrorAction SilentlyContinue }
        Write-Die "Could not move the binary to $dest : $($_.Exception.Message)"
    }

    Write-Ok "Binary installed at $dest"
    Add-ToUserPath $installDir
}

# ─── Main ───────────────────────────────────────────────────────────────────────

function Main {
    Write-Host ''
    Write-Host 'Pillbox — Installer' -ForegroundColor White
    Write-Host "https://github.com/$GithubRepo" -ForegroundColor DarkGray

    $arch       = Get-Arch
    $version    = Resolve-Version
    $installDir = Get-InstallDir

    Write-Host "  Platform: windows-$arch"
    Write-Host "  Version:  $version"
    Write-Host "  Binary:   $(Join-Path $installDir 'pillbox.exe')"

    Install-Binary $arch $version $installDir

    Write-Host ''
    Write-Host '  Pillbox installed successfully.' -ForegroundColor White
    Write-Host ''
    Write-Host '  Run ' -NoNewline; Write-Host 'pillbox --help' -ForegroundColor White -NoNewline; Write-Host ' to see all available commands.'
    Write-Host '  Full documentation at ' -NoNewline; Write-Host 'https://pillbox.dev/docs' -ForegroundColor White
    Write-Host ''
}

Main
