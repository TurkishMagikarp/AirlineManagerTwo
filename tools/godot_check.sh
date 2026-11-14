#!/usr/bin/env bash
# Run Godot tests and capture real UI screenshots on a Linux machine without a display.
#
#   tools/godot_check.sh test <name>        one headless test, e.g. "ui_layout" for tests/ui_layout_test.gd
#   tools/godot_check.sh tests              every tests/*_test.gd headless, one result line each
#   tools/godot_check.sh shots <out-dir>    ui_layout_test with real rendering; writes PNGs to <out-dir>
#
# Needs: a Godot 4.7 Linux binary (AIRLINE_GODOT, default /paperclip/airline/godot/godot), Xvfb and Mesa
# for "shots". Extra shared libraries next to the binary in lib/ are picked up automatically.
# The PowerShell restart tests (tests/run_*_restart_test.ps1) and START.bat are Windows-only and not covered.
set -euo pipefail

cd "$(dirname "$0")/.."
GODOT="${AIRLINE_GODOT:-/paperclip/airline/godot/godot}"
[ -x "$GODOT" ] || { echo "Godot binary not found at $GODOT (set AIRLINE_GODOT)"; exit 2; }
GODOT_LIB="$(dirname "$(readlink -f "$GODOT")")/lib"
[ -d "$GODOT_LIB" ] && export LD_LIBRARY_PATH="$GODOT_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

prepare() {
  cargo build --locked --manifest-path rust/Cargo.toml 2>&1 | tail -3
  # Import whenever the cache is missing; a stale cache only matters for new resources, so re-import then too.
  if [ ! -d .godot ] || [ -n "$(find resources scenes -newer .godot -print -quit 2>/dev/null)" ]; then
    "$GODOT" --headless --path . --import >/dev/null 2>&1 || true
    touch .godot
  fi
}

run_test() {
  local name="$1" log status=0
  [ -f "tests/${name}_test.gd" ] || { echo "MISSING tests/${name}_test.gd"; return 2; }
  log="$(mktemp)"
  timeout "${GODOT_TEST_TIMEOUT:-600}" "$GODOT" --headless --audio-driver Dummy --path . --script "res://tests/${name}_test.gd" >"$log" 2>&1 || status=$?
  if [ "$status" -eq 0 ]; then
    echo "ok   ${name}: $(grep -m1 -E '^PASS' "$log" || echo 'exit 0')"
  else
    echo "FAIL ${name} (exit ${status})"
    grep -E 'FAIL|ERROR|SCRIPT ERROR|Assertion|panicked' "$log" | head -15 || true
    tail -5 "$log"
  fi
  rm -f "$log"
  return "$status"
}

case "${1:-}" in
  test)
    [ -n "${2:-}" ] || { echo "usage: $0 test <name>"; exit 2; }
    prepare
    run_test "$2"
    ;;
  tests)
    prepare
    failed=0
    for file in tests/*_test.gd; do
      name="$(basename "$file" _test.gd)"
      case "$name" in
        *_restart) echo "skip ${name}: needs the Windows restart driver (tests/run_*_restart_test.ps1)"; continue ;;
      esac
      run_test "$name" || failed=$((failed + 1))
    done
    echo "failed: ${failed}"
    [ "$failed" -eq 0 ]
    ;;
  shots)
    out="${2:-}"
    [ -n "$out" ] || { echo "usage: $0 shots <out-dir>"; exit 2; }
    command -v Xvfb >/dev/null || { echo "Xvfb is not installed"; exit 2; }
    prepare
    mkdir -p "$out"
    out="$(cd "$out" && pwd)"
    display=""
    for n in 90 91 92 93 94 95 96 97 98 99; do
      [ -e "/tmp/.X11-unix/X${n}" ] || { display=":${n}"; break; }
    done
    [ -n "$display" ] || { echo "no free X display between :90 and :99"; exit 2; }
    Xvfb "$display" -screen 0 1600x1100x24 -nolisten tcp >/dev/null 2>&1 &
    xvfb_pid=$!
    trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
    sleep 2
    status=0
    DISPLAY="$display" timeout "${GODOT_TEST_TIMEOUT:-600}" "$GODOT" --audio-driver Dummy --rendering-driver opengl3 --path . \
      --script res://tests/ui_layout_test.gd -- "--capture-dir=${out}" 2>&1 | grep -E '^(PASS|FAIL)|SCRIPT ERROR|ERROR:' | head -20 || status=$?
    count="$(find "$out" -maxdepth 1 -name '*.png' | wc -l)"
    echo "screenshots: ${count} in ${out}"
    [ "$count" -gt 0 ]
    ;;
  *)
    sed -n 2,10p "$0"
    exit 2
    ;;
esac
