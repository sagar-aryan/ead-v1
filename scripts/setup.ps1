# EAD V1 dashboard: fetch, check dependencies, and run — Windows 10/11.
#
#   powershell -ExecutionPolicy Bypass -File setup.ps1          install and start
#   powershell -ExecutionPolicy Bypass -File setup.ps1 build    produce an installer
#   powershell -ExecutionPolicy Bypass -File setup.ps1 check    only report dependencies
#
# The repository is private: cloning needs either the GitHub CLI signed in
# (`gh auth login`) or Git Credential Manager, which opens a browser sign-in.
# Missing dependencies are reported with the command that installs them.
param([string]$Mode = "dev")
$ErrorActionPreference = "Stop"

$Repo = "sagar-aryan/ead-v1"
$Dir = if ($env:EAD_DIR) { $env:EAD_DIR } else { Join-Path $HOME "ead-v1" }
$script:missing = $false

function Ok($what)            { Write-Host "  ok      $what" }
function Need($what, $how)    { Write-Host "  MISSING $what`n          install: $how" -ForegroundColor Yellow; $script:missing = $true }
function Opt($what, $how)     { Write-Host "  --      $what (optional: $how)" }
function Has($cmd)            { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

Write-Host "Checking dependencies (Windows)"

if (Has git) { Ok "git" } else { Need "git" "winget install Git.Git" }

if (Has node) {
    $v = [version]((node -v).TrimStart("v"))
    if ($v -ge [version]"20.0.0") { Ok "node $v" } else { Need "node >= 20 (found $v)" "winget install OpenJS.NodeJS.LTS" }
} else { Need "node >= 20" "winget install OpenJS.NodeJS.LTS" }

if (Has rustc) {
    $v = [version]((rustc --version).Split(" ")[1])
    if ($v -ge [version]"1.92.0") { Ok "rust $v" } else { Need "rust >= 1.92 (found $v)" "rustup update stable" }
} else { Need "rust >= 1.92" "winget install Rustlang.Rustup" }

# Rust on Windows links with the MSVC toolchain; Tauri renders with WebView2.
# https://v2.tauri.app/start/prerequisites/#windows
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$msvc = (Test-Path $vswhere) -and (& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if ($msvc) { Ok "MSVC C++ build tools" } else {
    Need "MSVC C++ build tools" 'winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
}

$webview = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if ((Test-Path $webview) -or (Test-Path ($webview -replace "WOW6432Node\\", ""))) { Ok "WebView2 runtime" }
else { Need "WebView2 runtime" "winget install Microsoft.EdgeWebView2Runtime" }

# Only for the command-line tools in tools\, not for the dashboard.
if (Has python) {
    python -c "import serial" 2>$null; if ($LASTEXITCODE -eq 0) { Ok "python + pyserial" }
    else { Opt "pyserial" "python -m pip install pyserial   (tools\eadprobe.py over USB)" }
} else { Opt "python" "winget install Python.Python.3.12   (tools\eadprobe.py)" }
if (-not (Has pio)) { Opt "PlatformIO" "python -m pip install platformio   (only to rebuild/flash firmware)" }

if ($Mode -eq "check") { exit [int]$script:missing }
if ($script:missing) { Write-Host "`nInstall the MISSING items above, open a new terminal, and run this again."; exit 1 }

Write-Host "`nFetching $Repo into $Dir"
if (Test-Path (Join-Path $Dir ".git")) { git -C $Dir pull --ff-only }
elseif (Has gh) { gh repo clone $Repo $Dir }
else { git clone "https://github.com/$Repo.git" $Dir }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Set-Location (Join-Path $Dir "dashboard")
Write-Host "`nInstalling JavaScript dependencies"
npm ci; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($Mode -eq "build") {
    Write-Host "`nBuilding the installer (first build compiles Rust: several minutes)"
    npx tauri build
    Write-Host "`nInstallers are in $Dir\dashboard\src-tauri\target\release\bundle\"
} else {
    Write-Host "`nStarting the dashboard (first start compiles Rust: several minutes)"
    npx tauri dev
}
