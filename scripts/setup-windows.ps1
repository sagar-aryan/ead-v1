# EAD V1 dashboard on Windows 10 or 11 (x64): check dependencies, install what
# is missing, fetch the code and start the dashboard.
#
# Straight from GitHub (PowerShell). Downloaded to a file and run in its own
# process, so the script's `exit` cannot close your terminal:
#   irm https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-windows.ps1 -OutFile $env:TEMP\ead-setup.ps1
#   powershell -ExecutionPolicy Bypass -File $env:TEMP\ead-setup.ps1
#
#   ... setup-windows.ps1            check, install what is missing, clone/update, start
#   ... setup-windows.ps1 check      only report dependencies; change nothing
#   ... setup-windows.ps1 build      as the default, but produce an installer (.exe)
#
# Written for Windows PowerShell 5.1, which every Windows 10/11 has. Asks once
# before installing; installs use winget, and Windows may ask for administrator
# approval for the Visual Studio Build Tools.
param([string]$Mode = "dev")
if ($Mode -eq "install") { $Mode = "dev" }   # the word earlier instructions used
# Continue, not Stop: in Windows PowerShell 5.1 a native command's redirected
# stderr becomes a terminating error under Stop, so a failed `import serial`
# would abort the whole script. Native commands are checked by $LASTEXITCODE.
$ErrorActionPreference = "Continue"

$Repo = "sagar-aryan/ead-v1"
$Dir = if ($env:EAD_DIR) { $env:EAD_DIR } else { Join-Path $HOME "ead-v1" }
$script:Missing = @()

function Ok($what)         { Write-Host "  ok      $what" }
function Opt($what, $how)  { Write-Host "  --      $what (optional: $how)" }
function Need($what, $how, $winget) {
    Write-Host "  MISSING $what" -ForegroundColor Yellow
    Write-Host "          install: $how"
    $script:Missing += ,@($what, $winget)
}
function Has($cmd) { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

# winget and rustup change PATH in the registry; this shell only sees it if
# it is read back.
function Refresh-Path {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                [Environment]::GetEnvironmentVariable("Path", "User") + ";" +
                (Join-Path $HOME ".cargo\bin")
}

function Check {
    $script:Missing = @()
    Write-Host "Checking dependencies (Windows $([Environment]::OSVersion.Version), $env:PROCESSOR_ARCHITECTURE)"

    if (Has winget) { Ok "winget" }
    else { Opt "winget" "install 'App Installer' from the Microsoft Store; used to install whatever is missing" }

    if (Has git) { Ok "git" }
    else { Need "git" "winget install --id Git.Git -e" "Git.Git" }

    if (Has node) {
        $v = [version]((node -v).TrimStart("v"))
        if ($v -ge [version]"20.0.0") { Ok "node $v" }
        else { Need "node >= 20 (found $v)" "winget install --id OpenJS.NodeJS.LTS -e" "OpenJS.NodeJS.LTS" }
    } else { Need "node >= 20" "winget install --id OpenJS.NodeJS.LTS -e" "OpenJS.NodeJS.LTS" }

    if (Has rustc) {
        $v = [version]((rustc --version).Split(" ")[1])
        $hostTriple = (rustc -vV | Select-String "^host:").ToString()
        # krilla, which writes the PDF report, needs 1.92.
        if ($v -lt [version]"1.92.0") { Need "rust >= 1.92 (found $v)" "rustup update stable" "" }
        elseif ($hostTriple -notmatch "msvc") {
            # Tauri on Windows links with MSVC; the GNU toolchain will not build it.
            Need "rust MSVC toolchain (found $hostTriple)" "rustup default stable-msvc" ""
        } else { Ok "rust $v (msvc)" }
    } else { Need "rust >= 1.92" "winget install --id Rustlang.Rustup -e" "Rustlang.Rustup" }

    # Rust needs the MSVC linker and Windows SDK; the Build Tools provide both.
    # https://v2.tauri.app/start/prerequisites/#windows
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    $vc = $null
    if (Test-Path $vswhere) {
        $vc = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    }
    if ($vc) { Ok "MSVC C++ build tools" }
    else {
        Need "MSVC C++ build tools" `
             'winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"' `
             "Microsoft.VisualStudio.2022.BuildTools"
    }

    # Windows 11 always has WebView2; Windows 10 usually does, via Edge.
    $wv = "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    $wvUser = "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    if ((Test-Path "HKLM:\$wv") -or (Test-Path "HKCU:\$wvUser")) { Ok "WebView2 runtime" }
    else { Need "WebView2 runtime" "winget install --id Microsoft.EdgeWebView2Runtime -e" "Microsoft.EdgeWebView2Runtime" }

    # The device is a native USB CDC port (COMn); Windows 10/11 needs no driver.
    # Python is only for the command-line tools in tools\, not the dashboard.
    $py = $false
    if (Has python) { python -c "import sys" 2>$null; $py = ($LASTEXITCODE -eq 0) }
    if ($py) {
        python -c "import serial" 2>$null
        if ($LASTEXITCODE -eq 0) { Ok "python + pyserial" }
        else { Opt "pyserial" "python -m pip install pyserial   (tools\eadprobe.py)" }
    } else { Opt "python" "winget install --id Python.Python.3.12 -e   (tools\eadprobe.py)" }
    if (-not (Has pio)) { Opt "PlatformIO" "python -m pip install platformio   (only to reflash firmware)" }
}

function Install-Missing {
    foreach ($item in $script:Missing) {
        $what = $item[0]; $id = $item[1]
        Write-Host "`n== Installing $what"
        if (-not $id) {
            # Rust is present but too old, or the GNU toolchain: rustup fixes both.
            if ($what -like "*MSVC toolchain*") { rustup default stable-msvc } else { rustup update stable }
            continue
        }
        if (-not (Has winget)) {
            Write-Host "winget is not available. Install 'App Installer' from the Microsoft Store, or install $what by hand."
            continue
        }
        if ($id -eq "Microsoft.VisualStudio.2022.BuildTools") {
            winget install --id $id -e --accept-package-agreements --accept-source-agreements `
                --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
        } else {
            winget install --id $id -e --accept-package-agreements --accept-source-agreements
        }
        Refresh-Path
        if ($id -eq "Rustlang.Rustup" -and (Has rustup)) { rustup default stable-msvc }
    }
    Refresh-Path
}

Refresh-Path
Check
if ($Mode -eq "check") { if ($script:Missing.Count) { exit 1 } else { exit 0 } }

if ($script:Missing.Count) {
    $answer = Read-Host "`nInstall the $($script:Missing.Count) missing item(s) above now? [Y/n]"
    if ($answer -match "^[Nn]") { Write-Host "Nothing installed."; exit 1 }
    Install-Missing
    Write-Host ""
    Check
    if ($script:Missing.Count) {
        Write-Host "`nSomething is still missing; see above. Open a NEW PowerShell window and run this again."
        exit 1
    }
}

Write-Host "`nFetching $Repo into $Dir"
if (Test-Path (Join-Path $Dir ".git")) { git -C $Dir pull --ff-only }
else { git clone "https://github.com/$Repo.git" $Dir }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# `tauri dev` serves the UI on port 1420. If it is taken, a dashboard is almost
# certainly open already; say so, rather than fail later with a port error.
if ($Mode -eq "dev" -and (Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue)) {
    Write-Host "`nThe dashboard is already running (port 1420 is in use). Close it and run this again."
    exit 1
}

Set-Location (Join-Path $Dir "dashboard")
Write-Host "`nInstalling JavaScript dependencies"
npm ci
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($Mode -eq "build") {
    # NSIS, not MSI: the MSI path runs WiX, whose light.exe fails on recent
    # Windows 11 builds where VBScript is not installed.
    Write-Host "`nBuilding the installer (first build compiles Rust: several minutes)"
    npx tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "`nInstaller: $Dir\dashboard\src-tauri\target\release\bundle\nsis\"
    Write-Host "It is unsigned, so SmartScreen will warn: More info > Run anyway."
} else {
    Write-Host "`nStarting the dashboard (first start compiles Rust: several minutes)"
    Write-Host "Next time, just run this script again, or: cd $Dir\dashboard; npx tauri dev"
    npx tauri dev
}
