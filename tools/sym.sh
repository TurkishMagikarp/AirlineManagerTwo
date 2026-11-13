#!/usr/bin/env bash
# Usage: tools/sym.sh <symbol> [lines=40]   — print definition(s) of a symbol from CODEMAP, no grep needed.
cd "$(dirname "$0")/.." || exit 1
sym="$1"; n="${2:-40}"
[ -z "$sym" ] && { echo "usage: sym.sh <symbol> [lines]"; exit 1; }
awk -v s="$sym" -v n="$n" '
/^## /{f=$2; next}
f!="" && $1 ~ /^[0-9]+$/ {
  line=$0; sub(/^[0-9]+ /,"",line)
  if (match(line, "(^|[^A-Za-z0-9_])" s "([^A-Za-z0-9_]|$)")) print f, $1, n
}' docs/CODEMAP.md | while read -r f l c; do
  echo "=== $f:$l"; sed -n "${l},$((l+c-1))p" "$f"
done
