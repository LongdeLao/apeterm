#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
  ROOT_DIR=""
fi
APP_NAME="apeterm"
PREFIX="${PREFIX:-$HOME/.local}"
DEFAULT_BIN_DIR="$PREFIX/bin"
BIN_DIR="${BIN_DIR:-}"
SHARE_DIR="${SHARE_DIR:-$PREFIX/share/$APP_NAME}"
SCRIPT_DIR="$SHARE_DIR/scripts"
VERSION="${VERSION:-latest}"
GITHUB_REPO="${GITHUB_REPO:-LongdeLao/apeterm}"
REPO_REF="${REPO_REF:-master}"
BUILD_FROM_SOURCE="${BUILD_FROM_SOURCE:-0}"
INSTALL_PYTHON_DEPS="${INSTALL_PYTHON_DEPS:-1}"
PYTHON_BIN="${PYTHON_BIN:-}"
REAL_BIN="$SHARE_DIR/$APP_NAME-bin"
PATH_UPDATED=0

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 1
  }
}

pick_bin_dir() {
  if [[ -n "${BIN_DIR:-}" ]]; then
    return
  fi

  local candidate
  IFS=':' read -r -a path_parts <<< "$PATH"
  for candidate in "${path_parts[@]}"; do
    [[ -n "$candidate" ]] || continue
    [[ -d "$candidate" ]] || continue
    [[ -w "$candidate" ]] || continue
    BIN_DIR="$candidate"
    return
  done

  BIN_DIR="$DEFAULT_BIN_DIR"
}

shell_profile() {
  case "${SHELL:-}" in
    */zsh) printf '%s\n' "$HOME/.zshrc" ;;
    */bash) printf '%s\n' "$HOME/.bashrc" ;;
    */fish) printf '%s\n' "$HOME/.config/fish/config.fish" ;;
    *) printf '%s\n' "$HOME/.profile" ;;
  esac
}

ensure_bin_dir_on_path() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) return 0 ;;
  esac

  local profile line
  profile="$(shell_profile)"
  mkdir -p "$(dirname "$profile")"

  if [[ "$profile" == *"/config.fish" ]]; then
    line="fish_add_path \"$BIN_DIR\""
  else
    line="export PATH=\"$BIN_DIR:\$PATH\""
  fi

  if [[ ! -f "$profile" ]] || ! grep -Fqx "$line" "$profile"; then
    printf '\n%s\n' "$line" >> "$profile"
  fi
  PATH_UPDATED=1
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) os="apple-darwin" ;;
    Linux) os="unknown-linux-gnu" ;;
    *) echo "error: unsupported OS: $os" >&2; exit 1 ;;
  esac

  if [[ "$os" == "apple-darwin" ]]; then
    case "$arch" in
      arm64|aarch64) arch="aarch64" ;;
      x86_64)
        echo "error: Intel macOS is not supported by this installer" >&2
        echo "supported targets: Apple Silicon macOS, x86_64 Linux" >&2
        exit 1
        ;;
      *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
    esac
  else
    case "$arch" in
      x86_64) arch="x86_64" ;;
      *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
    esac
  fi

  printf '%s-%s\n' "$arch" "$os"
}

release_url() {
  local target version_tag
  target="$(detect_target)"
  version_tag="$VERSION"
  if [[ "$version_tag" != "latest" && "$version_tag" != v* ]]; then
    version_tag="v$version_tag"
  fi

  if [[ -z "$GITHUB_REPO" ]]; then
    echo "error: GITHUB_REPO must be set for prebuilt installs, e.g. GITHUB_REPO=owner/apeterm" >&2
    exit 1
  fi

  if [[ "$VERSION" == "latest" ]]; then
    printf 'https://github.com/%s/releases/latest/download/%s-%s.tar.gz\n' \
      "$GITHUB_REPO" "$APP_NAME" "$target"
  else
    printf 'https://github.com/%s/releases/download/%s/%s-%s.tar.gz\n' \
      "$GITHUB_REPO" "$version_tag" "$APP_NAME" "$target"
  fi
}

raw_url() {
  local path="$1"
  printf 'https://raw.githubusercontent.com/%s/%s/%s\n' \
    "$GITHUB_REPO" "$REPO_REF" "$path"
}

python_is_supported() {
  local python_bin="$1"
  "$python_bin" - <<'EOF' >/dev/null 2>&1
import sys
raise SystemExit(0 if sys.version_info >= (3, 10) else 1)
EOF
}

resolve_python_bin() {
  if [[ -n "$PYTHON_BIN" ]]; then
    need_cmd "$PYTHON_BIN"
    if ! python_is_supported "$PYTHON_BIN"; then
      echo "error: $PYTHON_BIN must be Python 3.10 or newer" >&2
      exit 1
    fi
    return
  fi

  local candidate
  for candidate in \
    /opt/homebrew/bin/python3 \
    python3.14 \
    python3.13 \
    python3.12 \
    python3.11 \
    python3.10 \
    python3 \
    python
  do
    if command -v "$candidate" >/dev/null 2>&1 && python_is_supported "$candidate"; then
      PYTHON_BIN="$(command -v "$candidate")"
      return
    fi
  done

  echo "error: ApeTerm requires Python 3.10+ to install bundled market data dependencies" >&2
  echo "install Python and rerun, or set PYTHON_BIN=/path/to/python3.10+" >&2
  exit 1
}

install_python_deps() {
  [[ "$INSTALL_PYTHON_DEPS" == "1" ]] || return 0

  resolve_python_bin
  "$PYTHON_BIN" -m venv "$SHARE_DIR/.venv"
  "$SHARE_DIR/.venv/bin/pip" install --upgrade pip >/dev/null
  "$SHARE_DIR/.venv/bin/pip" install -r "$SCRIPT_DIR/requirements.txt" >/dev/null
}

install_support_files() {
  mkdir -p "$SCRIPT_DIR"
  if [[ -n "$ROOT_DIR" ]]; then
    install -m 0644 "$ROOT_DIR/scripts/yfinance_stream.py" "$SCRIPT_DIR/yfinance_stream.py"
    install -m 0644 "$ROOT_DIR/scripts/yfinance_details.py" "$SCRIPT_DIR/yfinance_details.py"
    install -m 0644 "$ROOT_DIR/scripts/requirements.txt" "$SCRIPT_DIR/requirements.txt"
  else
    need_cmd curl
    curl -fsSL "$(raw_url scripts/yfinance_stream.py)" -o "$SCRIPT_DIR/yfinance_stream.py"
    curl -fsSL "$(raw_url scripts/yfinance_details.py)" -o "$SCRIPT_DIR/yfinance_details.py"
    curl -fsSL "$(raw_url scripts/requirements.txt)" -o "$SCRIPT_DIR/requirements.txt"
  fi
}

install_wrapper() {
  mkdir -p "$BIN_DIR"
  cat > "$BIN_DIR/$APP_NAME" <<EOF
#!/usr/bin/env bash
set -euo pipefail
export APETERM_SCRIPT_DIR="$SCRIPT_DIR"
if [[ -x "$SHARE_DIR/.venv/bin/python" ]]; then
  export APETERM_PYTHON="$SHARE_DIR/.venv/bin/python"
fi
exec "$REAL_BIN" "\$@"
EOF
  chmod +x "$BIN_DIR/$APP_NAME"
}

install_from_release() {
  need_cmd curl
  need_cmd tar

  local tmp url
  tmp="$(mktemp -d)"
  url="$(release_url)"

  mkdir -p "$BIN_DIR" "$SHARE_DIR"
  echo "downloading $url"
  if ! curl -fL "$url" -o "$tmp/$APP_NAME.tar.gz"; then
    echo "error: no published release asset for $(detect_target)" >&2
    echo "expected: $url" >&2
    echo "publish a GitHub release for this target, or run a source install from a local checkout." >&2
    exit 1
  fi
  tar -xzf "$tmp/$APP_NAME.tar.gz" -C "$tmp"

  local binary
  binary="$(find "$tmp" -type f -name "$APP_NAME" -perm -111 | head -n 1)"
  if [[ -z "$binary" ]]; then
    echo "error: release archive did not contain $APP_NAME" >&2
    exit 1
  fi

  install -m 0755 "$binary" "$REAL_BIN"
  install_support_files
  install_python_deps
  install_wrapper
  rm -rf "$tmp"
}

install_from_source() {
  need_cmd cargo
  if [[ -z "$ROOT_DIR" ]]; then
    echo "error: source install requires a checked out repository" >&2
    exit 1
  fi
  mkdir -p "$BIN_DIR" "$SHARE_DIR"
  cd "$ROOT_DIR"
  cargo build --release
  install -m 0755 "$ROOT_DIR/target/release/$APP_NAME" "$REAL_BIN"
  install_support_files
  install_python_deps
  install_wrapper
}

print_path_note() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
      echo "note: $BIN_DIR is not currently on PATH" >&2
      echo "add this to your shell profile:" >&2
      echo "  export PATH=\"$BIN_DIR:\$PATH\"" >&2
      ;;
  esac
}

main() {
  pick_bin_dir
  if [[ "$BUILD_FROM_SOURCE" == "1" ]]; then
    install_from_source
  else
    install_from_release
  fi

  ensure_bin_dir_on_path

  echo "installed $APP_NAME to $BIN_DIR/$APP_NAME"
  echo "runtime files in $SHARE_DIR"
  if [[ "$INSTALL_PYTHON_DEPS" == "1" ]]; then
    echo "installed python runtime deps to $SHARE_DIR/.venv"
  fi
  if [[ "$PATH_UPDATED" == "1" ]]; then
    echo "restart your shell, then run: $APP_NAME"
  else
    echo "run: $APP_NAME"
  fi
}

main "$@"
