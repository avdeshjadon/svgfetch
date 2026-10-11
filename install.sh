#!/bin/sh
#
# svgfetch — one-line installer (POSIX sh: macOS, Linux, Windows Git Bash).
#
#   macOS / Linux / Windows-Git-Bash:
#     curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | sh
#
#   Install somewhere else:
#     curl -fsSL .../install.sh | sh -s -- --dir "$HOME/bin"
#
# Always installs the latest release automatically (or pin with
# SVGFETCH_VERSION, e.g. SVGFETCH_VERSION=v0.2.3). Both `svgfetch` and aliases
# are installed to ~/.local/bin (or --dir), SHA-256 verified.
#
# Presentation only: the animated progress is cosmetic and is disabled when
# stderr is not a TTY, when NO_COLOR is set, or when TERM=dumb. The install
# directory is still printed to stdout as the final line.
set -eu

REPO="avdeshjadon/svgfetch"
BIN="svgfetch"
VERSION="${SVGFETCH_VERSION:-${GET_SVG_VERSION:-latest}}"

DIR="${INSTALL_DIR:-}"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --dir) DIR="$2"; shift 2 ;;
    --dir=*) DIR="${1#*=}"; shift ;;
    -h|--help) echo "usage: install.sh [--dir DIR]  (or set INSTALL_DIR / SVGFETCH_VERSION)"; exit 0 ;;
    *) echo "install.sh: unknown argument: $1" >&2; exit 1 ;;
  esac
done
[ -n "$DIR" ] || DIR="${HOME}/.local/bin"

command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }

# --- terminal capability detection ----------------------------------------
# All progress goes to stderr, so that is what we probe.
USE_COLOR=0
USE_ANIM=0
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ] && [ "${TERM:-}" != "dumb" ]; then
  USE_COLOR=1
  USE_ANIM=1
fi

# --- colour helpers --------------------------------------------------------
paint() {
  if [ "$USE_COLOR" = 1 ]; then
    printf '\033[%sm%s\033[0m' "$1" "$2"
  else
    printf '%s' "$2"
  fi
}
accent() { paint "1;38;5;209" "$1"; }
green()  { paint "32" "$1"; }
yellow() { paint "33" "$1"; }
red()    { paint "31" "$1"; }
dim()    { paint "2" "$1"; }

err() { printf 'error: %s\n' "$1" >&2; exit 1; }

repeat() { # repeat <char> <count>
  _i=0
  _s=''
  while [ "$_i" -lt "$2" ]; do
    _s="$_s$1"
    _i=$((_i + 1))
  done
  printf '%s' "$_s"
}

# Accent-bordered panel whose width tracks the longest line.
print_box() {
  _w=0
  for _l in "$@"; do
    _n=${#_l}
    if [ "$_n" -gt "$_w" ]; then _w=$_n; fi
  done
  _inner=$((_w + 4))
  _bar="$(repeat '─' "$_inner")"
  printf '  %s%s%s\n' "$(accent '┌')" "$(accent "$_bar")" "$(accent '┐')" >&2
  for _l in "$@"; do
    _pad=$((_w - ${#_l}))
    printf '  %s  %s%s  %s\n' "$(accent '│')" "$_l" "$(repeat ' ' "$_pad")" "$(accent '│')" >&2
  done
  printf '  %s%s%s\n' "$(accent '└')" "$(accent "$_bar")" "$(accent '┘')" >&2
}

# --- cursor / animation ----------------------------------------------------
CURSOR_HIDDEN=0
SPIN_PID=""
SPIN_TENTHS=0

hide_cursor() {
  [ "$USE_ANIM" = 1 ] || return 0
  [ "$CURSOR_HIDDEN" = 0 ] || return 0
  printf '\033[?25l' >&2
  CURSOR_HIDDEN=1
}
show_cursor() {
  [ "$CURSOR_HIDDEN" = 1 ] || return 0
  printf '\r\033[2K\033[?25h' >&2
  CURSOR_HIDDEN=0
}

start_spin() { # start_spin <label>
  [ "$USE_ANIM" = 1 ] || return 0
  _label="$1"
  hide_cursor
  _tf="$TMP/ticks"
  : > "$_tf" 2>/dev/null || true
  # Background ticker: pulsing glyph, block scanner, live elapsed time.
  (
    set -- · ✢ ✳ ✶ ✻ ✽ ✻ ✶ ✳ ✢
    _n_glyph=$#
    _i=0
    while :; do
      _gi=$(( (_i / 2) % _n_glyph ))
      _k=0
      _g=''
      for _gl in "$@"; do
        if [ "$_k" = "$_gi" ]; then _g="$_gl"; break; fi
        _k=$((_k + 1))
      done
      _p=$(( _i % 18 ))
      if [ "$_p" -lt 10 ]; then _pos=$_p; else _pos=$(( 18 - _p )); fi
      _sc=''
      _c=0
      while [ "$_c" -lt 10 ]; do
        if [ "$_c" = "$_pos" ]; then _sgr='1;38;5;209'; _cell='■'
        elif [ "$_c" = "$((_pos - 1))" ]; then _sgr='38;5;173'; _cell='▪'
        elif [ "$_c" = "$((_pos - 2))" ]; then _sgr='38;5;130'; _cell='▪'
        else _sgr='38;5;238'; _cell='▪'; fi
        _sc="$_sc$(paint "$_sgr" "$_cell")"
        _c=$((_c + 1))
      done
      _t=$(( _i * 7 / 10 ))
      printf '\r\033[2K  %s %s…  %s  %s' \
        "$(accent "$_g")" "$_label" "$_sc" "$(dim "$((_t / 10)).$((_t % 10))s")" >&2
      printf '%s' "$_t" > "$_tf" 2>/dev/null || true
      sleep 0.07
      _i=$((_i + 1))
    done
  ) &
  SPIN_PID=$!
}

stop_spin() {
  [ -n "$SPIN_PID" ] || return 0
  kill "$SPIN_PID" 2>/dev/null || true
  wait "$SPIN_PID" 2>/dev/null || true
  SPIN_PID=""
  if [ -n "${TMP:-}" ] && [ -f "$TMP/ticks" ]; then
    SPIN_TENTHS="$(cat "$TMP/ticks" 2>/dev/null || echo 0)"
  else
    SPIN_TENTHS=0
  fi
  [ -n "$SPIN_TENTHS" ] || SPIN_TENTHS=0
  printf '\r\033[2K' >&2
}

# --- step tree -------------------------------------------------------------
STEP_TOTAL=4
STEP_N=0
TALLY_DONE=0
TALLY_SKIP=0
TALLY_WARN=0

step_begin() { # step_begin <label>
  _step_label="$1"
  _step_buf=''
  _step_child=0
  _step_warn=0
  start_spin "$1"
}

step_child() { # step_child <ok|skip|warn|err> <tag> <text>
  case "$1" in
    ok)   _sym='✓'; _sgr='32'; TALLY_DONE=$((TALLY_DONE + 1)) ;;
    skip) _sym='–'; _sgr='2';  TALLY_SKIP=$((TALLY_SKIP + 1)) ;;
    warn) _sym='!'; _sgr='33'; _step_warn=1; TALLY_WARN=$((TALLY_WARN + 1)) ;;
    *)    _sym='!'; _sgr='31'; _step_warn=1; TALLY_WARN=$((TALLY_WARN + 1)) ;;
  esac
  if [ "$_step_child" = 0 ]; then _branch='    ⎿ '; else _branch='      '; fi
  _step_child=$((_step_child + 1))
  _pad="$(printf '%-9s' "$2")"
  _step_buf="${_step_buf}${_branch}$(paint "$_sgr" "$_sym $_pad") $3
"
}

step_end() {
  stop_spin
  STEP_N=$((STEP_N + 1))
  if [ "$_step_warn" = 1 ]; then _dot="$(yellow '●')"; else _dot="$(green '●')"; fi
  if [ "$USE_ANIM" = 1 ]; then
    printf '  %s %s [%s/%s]  %s\n' \
      "$_dot" "$_step_label" "$STEP_N" "$STEP_TOTAL" \
      "$(dim "$((SPIN_TENTHS / 10)).$((SPIN_TENTHS % 10))s")" >&2
  else
    printf '  %s %s [%s/%s]\n' "$_dot" "$_step_label" "$STEP_N" "$STEP_TOTAL" >&2
  fi
  printf '%s' "$_step_buf" >&2
  printf '\n' >&2
  if [ -n "${TMP:-}" ] && [ -s "$TMP/err" ]; then
    cat "$TMP/err" >&2 || true
    : > "$TMP/err"
  fi
}

print_summary() {
  _warn=''
  if [ "$TALLY_WARN" -gt 0 ]; then
    _warn="$(yellow "! $TALLY_WARN warning(s)")"
  else
    _warn="$(dim "! $TALLY_WARN warnings")"
  fi
  printf '  %s  %s  %s\n' \
    "$(green "✓ $TALLY_DONE done")" \
    "$(dim "– $TALLY_SKIP skipped")" \
    "$_warn" >&2
  printf '\n' >&2
}

# --- platform detection ---------------------------------------------------
ARCH="$(uname -m 2>/dev/null || echo unknown)"
OS="$(uname -s 2>/dev/null || echo unknown)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      x86_64|amd64)  TARGET="x86_64-apple-darwin" ;;
      *) err "unsupported Apple CPU: $ARCH" ;;
    esac
    EXT="tar.gz"
    ;;
  Linux)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
      *) err "unsupported Linux CPU: $ARCH (only x86_64 builds)" ;;
    esac
    EXT="tar.gz"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-pc-windows-msvc" ;;
      *) err "unsupported Windows CPU: $ARCH" ;;
    esac
    EXT="zip"
    ;;
  *)
    err "unsupported OS: $OS"
    ;;
esac

# --- temp workspace + cleanup ---------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/svgfetch.XXXXXX")"
: > "$TMP/err"
CLEANED=0

cleanup() {
  [ "$CLEANED" = 0 ] || return 0
  CLEANED=1
  trap - EXIT INT TERM HUP 2>/dev/null || true
  stop_spin
  show_cursor
  if [ -s "$TMP/err" ]; then
    cat "$TMP/err" >&2 || true
  fi
  rm -rf "$TMP"
}
on_signal() { cleanup; exit "$1"; }
trap 'on_signal 130' INT
trap 'on_signal 143' TERM
trap 'on_signal 129' HUP
trap 'cleanup' EXIT

# --- step 1: resolve latest release tag -----------------------------------
step_begin "Resolving release"
if [ "$VERSION" = "latest" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>>"$TMP/err" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | sed -n '1p')"
  if [ -z "$VERSION" ]; then
    stop_spin
    err "could not determine the latest release tag"
  fi
fi
step_child ok "resolved" "$VERSION"
step_end

BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
ARTIFACT="$BIN-$TARGET.$EXT"

# --- step 2: download artifact (+ checksum file) --------------------------
step_begin "Downloading $ARTIFACT"
if ! curl -fsSL -o "$TMP/$ARTIFACT" "$BASE_URL/$ARTIFACT" 2>>"$TMP/err"; then
  # Fallback to legacy artifact name if downloading older release
  ARTIFACT="get-svg-$TARGET.$EXT"
  curl -fsSL -o "$TMP/$ARTIFACT" "$BASE_URL/$ARTIFACT" 2>>"$TMP/err"
fi
curl -fsSL -o "$TMP/$ARTIFACT.sha256" "$BASE_URL/$ARTIFACT.sha256" 2>>"$TMP/err"
step_child ok "download" "$ARTIFACT"
step_end

# --- step 3: verify SHA-256 -----------------------------------------------
HASHTOOL=''
if command -v sha256sum >/dev/null 2>&1; then
  HASHTOOL='sha256sum'
elif command -v shasum >/dev/null 2>&1; then
  HASHTOOL='shasum'
fi
sha256_file() {
  if [ "$HASHTOOL" = 'sha256sum' ]; then
    ( cd "$TMP" && sha256sum "$1" | awk '{print $1}' )
  else
    ( cd "$TMP" && shasum -a 256 "$1" | awk '{print $1}' )
  fi
}
step_begin "Verifying checksum"
if [ -z "$HASHTOOL" ]; then
  stop_spin
  err "no SHA-256 tool (sha256sum / shasum) available"
fi
EXPECTED="$(awk '{print $1}' "$TMP/$ARTIFACT.sha256")"
ACTUAL="$(sha256_file "$ARTIFACT")"
if [ "$EXPECTED" != "$ACTUAL" ]; then
  stop_spin
  err "checksum mismatch for $ARTIFACT"
fi
step_child ok "verified" "$ACTUAL"
step_end

# --- step 4: extract + install --------------------------------------------
step_begin "Installing"
EXTRACTED="$TMP/extracted"
mkdir -p "$EXTRACTED"
if [ "$EXT" = "zip" ]; then
  if ! command -v unzip >/dev/null 2>&1; then
    stop_spin
    err "'unzip' is required on Windows Git Bash"
  fi
  ( cd "$EXTRACTED" && unzip -q "$TMP/$ARTIFACT" ) 2>>"$TMP/err"
else
  tar -xzf "$TMP/$ARTIFACT" -C "$EXTRACTED" 2>>"$TMP/err"
fi

mkdir -p "$DIR"
PRIMARY_BINPATH="$(find "$EXTRACTED" -type f -name "svgfetch" -o -type f -name "svgfetch.exe" -o -type f -name "get-svg" -o -type f -name "getsvg" 2>/dev/null | sed -n '1p')"
if [ -z "$PRIMARY_BINPATH" ]; then
  stop_spin
  err "archive did not contain an executable binary"
fi

for NAME in "$BIN" "svg" "svg-fetch"; do
  BINPATH="$(find "$EXTRACTED" -type f -name "$NAME" -o -type f -name "$NAME.exe" 2>/dev/null | sed -n '1p')"
  if [ -z "$BINPATH" ]; then
    BINPATH="$PRIMARY_BINPATH"
  fi
  install -m 755 "$BINPATH" "$DIR/$NAME" 2>>"$TMP/err"
  step_child ok "installed" "$DIR/$NAME"
done
step_end

print_summary

if [ -x "$DIR/$BIN" ]; then
  "$DIR/$BIN" --version >/dev/null 2>&1 \
    && printf '  %s %s\n' "$(green '✓')" "$(dim "$("$DIR/$BIN" --version)")" >&2 || true
fi

case ":$PATH:" in
  *":$DIR:"*) : ;;
  *) printf '  %s add %s to your PATH, e.g.: export PATH="%s:$PATH"\n' "$(yellow '!')" "$DIR" "$DIR" >&2 ;;
esac

print_box \
  "svgfetch $VERSION is ready!" \
  "" \
  "Instant Vector Engine & Curated Icon Registry" \
  "" \
  "Quick Start:" \
  "  svgfetch                Launch interactive vector search" \
  "  svgfetch amazon         Download Amazon vector directly" \
  "  svgfetch --help         View documentation and options"

echo "$DIR"
