#!/usr/bin/env bash
# Usage: tools/sym.sh <symbol> [lines=40]   print the definition(s) of a symbol in rust/src and scripts
#        tools/sym.sh -u <text>             list where a text is used (file:line, at most 40 hits)
cd "$(dirname "$0")/.." || exit 1
if [ "$1" = "-u" ]; then
  [ -z "$2" ] && { echo "usage: sym.sh -u <text>"; exit 1; }
  grep -rnF --include='*.rs' --include='*.gd' -- "$2" rust/src scripts tests | cut -c1-160 | head -40
  exit 0
fi
sym="$1"; n="${2:-40}"
[ -z "$sym" ] && { echo "usage: sym.sh <symbol> [lines] | sym.sh -u <text>"; exit 1; }
defs() {
  grep -rnE --include='*.rs' --include='*.gd' \
    '^\s*(pub(\([a-z]+\))? )?(const fn|fn|struct|enum|trait|impl|mod|const|static|type|func|signal|class|class_name) ' rust/src scripts
}
res=$(defs | grep -iw -- "$sym")
[ -z "$res" ] && { echo "(no exact match; substring matches)" >&2; res=$(defs | grep -i -- "$sym"); }
[ -z "$res" ] && { echo "no definition found for $sym"; exit 1; }
echo "$res" | while IFS=: read -r f l _; do
  echo "=== $f:$l"; sed -n "${l},$((l+n-1))p" "$f"
done
