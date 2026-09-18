#!/usr/bin/env bash
# EAD V1 dashboard: fetch, check dependencies, and run — Linux (Debian/Ubuntu).
#
# Straight from GitHub, no download step:
#   curl -fsSL https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-linux.sh | bash -s -- check
#
#   bash setup-linux.sh            clone/update, install, and start the dashboard
#   bash setup-linux.sh build      the same, but produce an installer instead
#   bash setup-linux.sh check      only report which dependencies are present
#
# Missing dependencies are reported with the command that installs them; this
# script never runs sudo on its own.
set -euo pipefail

REPO="sagar-aryan/ead-v1"
DIR="${EAD_DIR:-$HOME/ead-v1}"
MODE="${1:-dev}"
missing=0

ok()   { printf '  ok      %s\n' "$1"; }
need() { printf '  MISSING %s\n          install: %s\n' "$1" "$2"; missing=1; }
opt()  { printf '  --      %s (optional: %s)\n' "$1" "$2"; }
has()  { command -v "$1" >/dev/null 2>&1; }

# Compares dotted versions: true when $1 >= $2.
at_least() { [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" = "$2" ]; }

echo "Checking dependencies (Linux)"

has git && ok "git $(git --version | awk '{print $3}')" || need git "sudo apt install git"

if has node; then
  v="$(node -v | tr -d v)"
  at_least "$v" 20.0.0 && ok "node $v" || need "node >= 20 (found $v)" "https://nodejs.org or nvm install 22"
else
  need "node >= 20" "https://nodejs.org or nvm install 22"
fi

if has rustc; then
  v="$(rustc --version | awk '{print $2}')"
  at_least "$v" 1.92.0 && ok "rust $v" || need "rust >= 1.92 (found $v)" "rustup update stable"
else
  need "rust >= 1.92" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

{
  # Tauri 2 on Linux: WebKitGTK 4.1 and the usual build chain.
  # https://v2.tauri.app/start/prerequisites/#linux
  pkgs="libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config"
  if has pkg-config && pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    ok "webkit2gtk 4.1"
  else
    need "Tauri system libraries (webkit2gtk 4.1 and friends)" "sudo apt install $pkgs"
  fi
  # The device is a USB serial port; reading it needs the dialout group.
  if id -nG | grep -qw dialout; then ok "in the dialout group (USB serial access)"
  else opt "dialout group" "sudo usermod -aG dialout \$USER, then log out and in"; fi
}

# Only for the command-line tools in tools/, not for the dashboard.
if has python3; then
  python3 -c 'import serial' 2>/dev/null && ok "python3 + pyserial" \
    || opt "pyserial" "python3 -m pip install pyserial   (tools/eadprobe.py over USB)"
  python3 -c 'import scipy' 2>/dev/null || opt "scipy" "python3 -m pip install scipy   (tools/check_mat.py)"
else
  opt "python3" "tools/eadprobe.py and the export checkers"
fi
has pio || opt "PlatformIO" "python3 -m pip install platformio   (only to rebuild/flash firmware)"

if [ "$MODE" = check ]; then exit "$missing"; fi
if [ "$missing" = 1 ]; then
  echo; echo "Install the MISSING items above, then run this script again."; exit 1
fi

echo; echo "Fetching $REPO into $DIR"
if [ -d "$DIR/.git" ]; then
  git -C "$DIR" pull --ff-only
else
  git clone "https://github.com/$REPO.git" "$DIR"
fi

cd "$DIR/dashboard"
echo; echo "Installing JavaScript dependencies"
npm ci

if [ "$MODE" = build ]; then
  echo; echo "Building the installer (first build compiles Rust: several minutes)"
  npx tauri build
  echo; echo "Installers are in $DIR/dashboard/src-tauri/target/release/bundle/"
else
  echo; echo "Starting the dashboard (first start compiles Rust: several minutes)"
  npx tauri dev
fi
