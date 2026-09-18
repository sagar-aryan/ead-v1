#!/usr/bin/env bash
# EAD V1 dashboard on Linux: check dependencies, install what is missing,
# fetch the code and start the dashboard. One command:
#
#   curl -fsSL https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-linux.sh | bash
#
# or, once downloaded:
#
#   bash setup-linux.sh            check, install what is missing, clone/update, start
#   bash setup-linux.sh check      only report dependencies; change nothing
#   bash setup-linux.sh build      as the default, but produce .deb/.rpm/.AppImage instead
#
# System packages are installed with apt (Debian, Ubuntu, Mint, Pop!_OS), dnf
# (Fedora) or pacman (Arch, Manjaro), after one confirmation; sudo asks for your
# password. Node and Rust are installed per user, through nvm and rustup, so no
# distribution's older packaged versions get in the way.
set -euo pipefail

REPO="sagar-aryan/ead-v1"
DIR="${EAD_DIR:-$HOME/ead-v1}"
MODE="${1:-dev}"
[ "$MODE" = install ] && MODE=dev   # the word earlier instructions used
NVM_VERSION="v0.40.1"
NODE_MAJOR=22
missing=()

ok()   { printf '  ok      %s\n' "$1"; }
need() { printf '  MISSING %s\n' "$1"; missing+=("$2"); }
opt()  { printf '  --      %s (optional: %s)\n' "$1" "$2"; }
has()  { command -v "$1" >/dev/null 2>&1; }

# True when dotted version $1 >= $2.
at_least() {
  awk -v a="$1" -v b="$2" 'BEGIN {
    split(a, x, "."); split(b, y, ".");
    for (i = 1; i <= 3; i++) { if (x[i] + 0 > y[i] + 0) exit 0; if (x[i] + 0 < y[i] + 0) exit 1 }
    exit 0 }'
}

# nvm and rustup install into the home directory; a non-interactive shell
# (and `curl | bash` is one) never reads the ~/.bashrc lines that add them.
load_user_tools() {
  export NVM_DIR="$HOME/.nvm"
  # shellcheck disable=SC1091
  [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm use --silent "$NODE_MAJOR" >/dev/null 2>&1 || true
  # shellcheck disable=SC1091
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env" || true
  hash -r
}

if has apt-get; then PM=apt
elif has dnf; then PM=dnf
elif has pacman; then PM=pacman
else PM=none; fi

# Tauri 2's system libraries, per distribution.
# https://v2.tauri.app/start/prerequisites/#linux
case "$PM" in
  apt) SYSTEM_PKGS="git curl wget file build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev" ;;
  dnf) SYSTEM_PKGS="git curl wget file gcc gcc-c++ make pkgconf-pkg-config webkit2gtk4.1-devel openssl-devel libxdo-devel libappindicator-gtk3-devel librsvg2-devel" ;;
  pacman) SYSTEM_PKGS="git curl wget file base-devel webkit2gtk-4.1 openssl xdotool libappindicator-gtk3 librsvg" ;;
  *) SYSTEM_PKGS="" ;;
esac

# The device is a USB serial port. Arch gives serial ports to `uucp`, everyone
# else to `dialout`; if the device is plugged in, ask the port itself.
SERIAL_GROUP=dialout
[ "$PM" = pacman ] && SERIAL_GROUP=uucp
for port in /dev/ttyACM*; do
  [ -e "$port" ] && SERIAL_GROUP="$(stat -c %G "$port")" && break
done

check() {
  missing=()
  echo "Checking dependencies (Linux, $PM)"

  if has git && has curl && has cc && has pkg-config && pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    ok "system libraries (git, compiler, webkit2gtk 4.1)"
  else
    need "system libraries (git, compiler, webkit2gtk 4.1 and the rest Tauri needs)" system
  fi

  if has node && at_least "$(node -v | tr -d v)" 20.0.0; then
    ok "node $(node -v | tr -d v)"
  else
    need "node >= 20$(has node && echo " (found $(node -v))")" node
  fi

  # krilla, which writes the PDF report, needs Rust 1.92.
  if has rustc && at_least "$(rustc --version | awk '{print $2}')" 1.92.0; then
    ok "rust $(rustc --version | awk '{print $2}')"
  else
    need "rust >= 1.92$(has rustc && echo " (found $(rustc --version | awk '{print $2}'))")" rust
  fi

  if id -nG | grep -qw "$SERIAL_GROUP"; then
    ok "in the $SERIAL_GROUP group (USB serial access)"
  else
    need "membership of the $SERIAL_GROUP group (to open the device's USB port)" serial
  fi

  # Only for the command-line tools in tools/, never needed by the dashboard.
  if has python3 && python3 -c 'import serial' 2>/dev/null; then ok "python3 + pyserial"
  else opt "pyserial" "tools/eadprobe.py; apt: python3-serial, dnf: python3-pyserial, pacman: python-pyserial"; fi
}

install_one() {
  case "$1" in
    system)
      case "$PM" in
        apt) sudo apt-get update && sudo apt-get install -y $SYSTEM_PKGS ;;
        dnf) sudo dnf install -y $SYSTEM_PKGS ;;
        pacman) sudo pacman -S --needed --noconfirm $SYSTEM_PKGS ;;
        *) echo "  No apt, dnf or pacman here. Install Tauri's prerequisites by hand:"
           echo "  https://v2.tauri.app/start/prerequisites/#linux"; return 1 ;;
      esac ;;
    node)
      if [ ! -s "$HOME/.nvm/nvm.sh" ]; then
        # nvm adds itself to ~/.bashrc, so `node` works in new terminals too.
        curl -fsSL "https://raw.githubusercontent.com/nvm-sh/nvm/$NVM_VERSION/install.sh" | bash
      fi
      export NVM_DIR="$HOME/.nvm"; . "$NVM_DIR/nvm.sh"
      nvm install "$NODE_MAJOR" && nvm alias default "$NODE_MAJOR" ;;
    rust)
      if has rustup; then
        rustup update stable && rustup default stable
      else
        # SKIP_PATH_CHECK: a distribution's packaged rustc may already be on
        # PATH; rustup's copy goes first once ~/.cargo/env is loaded.
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
          | RUSTUP_INIT_SKIP_PATH_CHECK=yes sh -s -- -y --default-toolchain stable
      fi ;;
    serial)
      sudo usermod -aG "$SERIAL_GROUP" "$USER"
      SERIAL_ADDED=1 ;;
  esac
}

load_user_tools
check
[ "$MODE" = check ] && exit $(( ${#missing[@]} > 0 ))

if [ ${#missing[@]} -gt 0 ]; then
  echo
  printf 'Install the %d missing item(s) above now? [Y/n] ' "${#missing[@]}"
  # From the terminal, not stdin: under `curl | bash` stdin is this script.
  # With no terminal at all (automation), default to yes: that is what this
  # script was asked to do.
  answer=y
  { read -r answer </dev/tty; } 2>/dev/null || { answer=y; echo; }
  if [[ "$answer" =~ ^[Nn] ]]; then echo "Nothing installed."; exit 1; fi
  for item in "${missing[@]}"; do
    echo; echo "== Installing $item"
    install_one "$item" || { echo "Installing $item failed; see the output above."; exit 1; }
  done
  load_user_tools
  echo; check
  # The group change is the one item that cannot take effect in this session.
  if [ ${#missing[@]} -eq 1 ] && [ "${missing[0]}" = serial ] && [ "${SERIAL_ADDED:-0}" = 1 ]; then
    missing=()
  fi
  if [ ${#missing[@]} -gt 0 ]; then
    echo; echo "Something is still missing; see above. Open a new terminal and run this again."
    exit 1
  fi
fi

echo; echo "Fetching $REPO into $DIR"
if [ -d "$DIR/.git" ]; then
  git -C "$DIR" pull --ff-only
else
  git clone "https://github.com/$REPO.git" "$DIR"
fi

# `tauri dev` serves the UI on port 1420. If it is taken, a dashboard is almost
# certainly open already; say so, rather than fail later with a port error.
if [ "$MODE" = dev ] && (: </dev/tcp/127.0.0.1/1420) 2>/dev/null; then
  echo; echo "The dashboard is already running (port 1420 is in use). Close it and run this again."
  exit 1
fi

cd "$DIR/dashboard"
echo; echo "Installing JavaScript dependencies"
npm ci

if [ "${SERIAL_ADDED:-0}" = 1 ]; then
  echo
  echo "NOTE: you were added to the $SERIAL_GROUP group. Log out and back in (or reboot)"
  echo "before the dashboard can open the device's USB port. Everything else works now."
fi

if [ "$MODE" = build ]; then
  echo; echo "Building installers (first build compiles Rust: several minutes)"
  npx tauri build
  echo; echo "Installers are in $DIR/dashboard/src-tauri/target/release/bundle/"
else
  echo; echo "Starting the dashboard (first start compiles Rust: several minutes)"
  echo "Next time, just run this script again, or: cd $DIR/dashboard && npx tauri dev"
  npx tauri dev
fi
