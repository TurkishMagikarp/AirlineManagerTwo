#!/usr/bin/env bash
# Usage: tools/sym.sh <symbol> [lines=40]   — print definition(s) of a symbol from CODEMAP, no grep needed.
cd "$(dirname "$0")/.." || exit 1
sym="$1"; n="${2:-40}"
[ -z "$sym" ] && { echo "usage: sym.sh <symbol> [lines]"; exit 1; }
find_syms() { awk -v s="$sym" -v n="$n" -v mode="$1" '
/^## /{f=$2; next}
f!="" && $1 ~ /^[0-9]+$/ {
  line=$0; sub(/^[0-9]+ /,"",line)
  pat = (mode=="exact") ? "(^|[^A-Za-z0-9_])" s "([^A-Za-z0-9_]|$)" : s
  if (match(tolower(line), tolower(pat))) print f, $1, n
}' docs/CODEMAP.md; }
res=$(find_syms exact); [ -z "$res" ] && { echo "(no exact match; substring matches)" >&2; res=$(find_syms sub); }
echo "$res" | while read -r f l c; do
  echo "=== $f:$l"; sed -n "${l},$((l+c-1))p" "$f"
done
