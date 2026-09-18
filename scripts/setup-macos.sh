#!/usr/bin/env bash
# EAD V1 dashboard on macOS (Apple Silicon or Intel, macOS 11 or newer).
#
#   bash setup-macos.sh            check dependencies, clone/update, start the dashboard
#   bash setup-macos.sh install    offer to install whatever is missing, then continue
#   bash setup-macos.sh check      only report dependencies
#   bash setup-macos.sh build      produce EAD Dashboard.app and a .dmg instead
#
# The repository is private: cloning needs the GitHub CLI signed in
# (`gh auth login`) or a personal access token when git asks for a password.
# Nothing is installed without asking first.
set -euo pipefail

REPO="sagar-aryan/ead-v1"
DIR="${EAD_DIR:-$HOME/ead-v1}"
MODE="${1:-dev}"
missing=()

ok()   { printf '  ok      %s\n' "$1"; }
need() { printf '  MISSING %s\n          install: %s\n' "$1" "$2"; missing+=("$1|$2"); }
opt()  { printf '  --      %s (optional: %s)\n' "$1" "$2"; }
has()  { command -v "$1" >/dev/null 2>&1; }

# True when dotted version $1 >= $2. Done by hand: the `sort` that ships with
# macOS is too old for `sort -V`.
at_least() {
  awk -v a="$1" -v b="$2" 'BEGIN {
    split(a, x, "."); split(b, y, ".");
    for (i = 1; i <= 3; i++) { if (x[i] + 0 > y[i] + 0) exit 0; if (x[i] + 0 < y[i] + 0) exit 1 }
    exit 0 }'
}

# rustup and Homebrew install into places a fresh shell has not got on PATH yet.
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
for brew in /opt/homebrew/bin/brew /usr/local/bin/brew; do
  [ -x "$brew" ] && eval "$("$brew" shellenv)" && break
done

check() {
  missing=()
  echo "Checking dependencies (macOS $(sw_vers -productVersion), $(uname -m))"

  # Clang, the linker and git all come with the command line tools.
  if xcode-select -p >/dev/null 2>&1; then ok "Xcode command line tools"
  else need "Xcode command line tools" "xcode-select --install"; fi

  has git && ok "git $(git --version | awk '{print $3}')" \
    || need "git" "xcode-select --install"

  has brew && ok "Homebrew" \
    || opt "Homebrew" 'only used to install node: https://brew.sh'

  if has node; then
    v="$(node -v | tr -d v)"
    at_least "$v" 20.0.0 && ok "node $v" || need "node >= 20 (found $v)" "brew install node"
  else
    need "node >= 20" "brew install node"
  fi

  if has rustc; then
    v="$(rustc --version | awk '{print $2}')"
    # krilla, which writes the PDF report, needs 1.92.
    at_least "$v" 1.92.0 && ok "rust $v" || need "rust >= 1.92 (found $v)" "rustup update stable"
  else
    need "rust >= 1.92" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
  fi

  # The device is a native USB CDC port, /dev/cu.usbmodem*: macOS needs no driver.
  if has python3; then
    python3 -c 'import serial' 2>/dev/null && ok "python3 + pyserial" \
      || opt "pyserial" "python3 -m pip install --user pyserial   (tools/eadprobe.py)"
  else
    opt "python3" "xcode-select --install   (tools/eadprobe.py)"
  fi
  has pio || opt "PlatformIO" "python3 -m pip install --user platformio   (only to reflash firmware)"
}

install_missing() {
  # ${a[@]+...}: macOS still ships bash 3.2, where an empty array is
  # "unbound" under set -u.
  for item in ${missing[@]+"${missing[@]}"}; do
    what="${item%%|*}"; how="${item#*|}"
    read -r -p "Install $what with: $how ? [y/N] " answer
    [[ "$answer" =~ ^[Yy] ]] || continue
    if [[ "$how" == "xcode-select --install" ]]; then
      xcode-select --install || true
      echo "A macOS dialog has opened. Finish it, then run this script again."
      exit 1
    fi
    if [[ "$how" == brew* ]] && ! has brew; then
      echo "Homebrew is not installed. Install it from https://brew.sh first."
      continue
    fi
    bash -c "$how"
  done
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  hash -r
}

check
[ "$MODE" = check ] && exit $(( ${#missing[@]} > 0 ))

if [ ${#missing[@]} -gt 0 ]; then
  if [ "$MODE" = install ]; then
    install_missing
    echo; check
  fi
  if [ ${#missing[@]} -gt 0 ]; then
    echo; echo "Install the MISSING items above (or run: bash setup-macos.sh install), then run this again."
    exit 1
  fi
fi

echo; echo "Fetching $REPO into $DIR"
if [ -d "$DIR/.git" ]; then
  git -C "$DIR" pull --ff-only
elif has gh; then
  gh repo clone "$REPO" "$DIR"
else
  git clone "https://github.com/$REPO.git" "$DIR"
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
  npx tauri dev
fi
