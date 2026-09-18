#!/usr/bin/env bash
# EAD V1 dashboard on macOS (Apple Silicon or Intel, macOS 11 or newer):
# check dependencies, install what is missing, fetch the code and start the
# dashboard. One command:
#
#   curl -fsSL https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-macos.sh | bash
#
# or, once downloaded:
#
#   bash setup-macos.sh            check, install what is missing, clone/update, start
#   bash setup-macos.sh check      only report dependencies; change nothing
#   bash setup-macos.sh build      as the default, but produce EAD Dashboard.app and a .dmg
#
# Asks once before installing. Node comes from Homebrew when it is installed,
# otherwise from nvm; Rust from rustup. Neither needs an administrator password.
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

# True when dotted version $1 >= $2. Done in awk: the `sort` that ships with
# macOS is too old for `sort -V`.
at_least() {
  awk -v a="$1" -v b="$2" 'BEGIN {
    split(a, x, "."); split(b, y, ".");
    for (i = 1; i <= 3; i++) { if (x[i] + 0 > y[i] + 0) exit 0; if (x[i] + 0 < y[i] + 0) exit 1 }
    exit 0 }'
}

# Homebrew, nvm and rustup install where a non-interactive shell (and
# `curl | bash` is one) has not got them on PATH.
load_user_tools() {
  for brew in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    if [ -x "$brew" ]; then eval "$("$brew" shellenv)"; break; fi
  done
  export NVM_DIR="$HOME/.nvm"
  # shellcheck disable=SC1091
  [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm use --silent "$NODE_MAJOR" >/dev/null 2>&1 || true
  # shellcheck disable=SC1091
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env" || true
  hash -r
}

check() {
  missing=()
  echo "Checking dependencies (macOS $(sw_vers -productVersion), $(uname -m))"

  # Clang, the linker and git all come with the command line tools.
  if xcode-select -p >/dev/null 2>&1 && has git; then ok "Xcode command line tools (compiler, git)"
  else need "Xcode command line tools (compiler, git)" xcode; fi

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

  # The device is a native USB CDC port, /dev/cu.usbmodem*: no driver needed.
  # Python is only for the command-line tools in tools/, never the dashboard.
  if has python3 && python3 -c 'import serial' 2>/dev/null; then ok "python3 + pyserial"
  else opt "pyserial" "python3 -m pip install --user pyserial   (tools/eadprobe.py)"; fi
}

install_one() {
  case "$1" in
    xcode)
      # This opens a macOS dialog and returns at once; nothing else can be
      # built until it finishes.
      xcode-select --install 2>/dev/null || true
      echo
      echo "A macOS dialog has opened to install the command line tools."
      echo "Click Install, wait for it to finish, then run this script again."
      exit 1 ;;
    node)
      if has brew; then
        brew install node
      else
        if [ ! -s "$HOME/.nvm/nvm.sh" ]; then
          # nvm adds itself to the shell's startup file, so `node` works in
          # new Terminal windows too. zsh is the macOS default; give nvm a
          # ~/.zshrc to write to if there is none yet.
          [ -f "$HOME/.zshrc" ] || touch "$HOME/.zshrc"
          curl -fsSL "https://raw.githubusercontent.com/nvm-sh/nvm/$NVM_VERSION/install.sh" | bash
        fi
        export NVM_DIR="$HOME/.nvm"; . "$NVM_DIR/nvm.sh"
        nvm install "$NODE_MAJOR" && nvm alias default "$NODE_MAJOR"
      fi ;;
    rust)
      if has rustup; then
        rustup update stable && rustup default stable
      else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      fi ;;
  esac
}

load_user_tools
check
[ "$MODE" = check ] && exit $(( ${#missing[@]} > 0 ))

if [ ${#missing[@]} -gt 0 ]; then
  echo
  printf 'Install the %d missing item(s) above now? [Y/n] ' "${#missing[@]}"
  # From the terminal, not stdin: under `curl | bash` stdin is this script.
  answer=y
  { read -r answer </dev/tty; } 2>/dev/null || { answer=y; echo; }
  if [[ "$answer" =~ ^[Nn] ]]; then echo "Nothing installed."; exit 1; fi
  # ${a[@]+...}: macOS still ships bash 3.2, where an empty array is
  # "unbound" under set -u.
  for item in ${missing[@]+"${missing[@]}"}; do
    echo; echo "== Installing $item"
    install_one "$item" || { echo "Installing $item failed; see the output above."; exit 1; }
  done
  load_user_tools
  echo; check
  if [ ${#missing[@]} -gt 0 ]; then
    echo; echo "Something is still missing; see above. Open a new Terminal window and run this again."
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

if [ "$MODE" = build ]; then
  echo; echo "Building EAD Dashboard.app and a .dmg (first build compiles Rust: several minutes)"
  npx tauri build --bundles app,dmg
  echo
  echo "Built into $DIR/dashboard/src-tauri/target/release/bundle/"
  echo "The app is unsigned. Built here it opens normally; copied to another Mac,"
  echo "open it the first time with right-click > Open."
else
  echo; echo "Starting the dashboard (first start compiles Rust: several minutes)"
  echo "Next time, just run this script again, or: cd $DIR/dashboard && npx tauri dev"
  npx tauri dev
fi
