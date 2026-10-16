#!/usr/bin/env bash
# Lists doc sections whose source files changed since BASE (default
# origin/main) while the section itself did not. Each section names its
# sources with \src{...} in its \stagebox line, so that line is the map.
#
#   docs/check-sync.sh [BASE]
#
# Exits 0 always; in GitHub Actions it prints a warning annotation per stale
# section so the PR shows which pages to revisit.
set -euo pipefail
cd "$(dirname "$0")/.."
base="${1:-origin/main}"
changed="$(git diff --name-only "$base"...HEAD)"
stale=0
for tex in docs/sections/*.tex; do
  box="$(grep -m1 '^\\stagebox' "$tex" || true)"
  [ -n "$box" ] || continue
  grep -qxF "$tex" <<<"$changed" && continue
  hits=""
  for src in $(grep -o '\\src{[^}]*}' <<<"$box" | sed 's/^\\src{//; s/}$//'); do
    src="${src//\\_/_}"
    src="${src%/}"
    if grep -q -e "^$src\$" -e "^$src/" <<<"$changed"; then hits="$hits $src"; fi
  done
  if [ -n "$hits" ]; then
    stale=1
    msg="$tex may be out of date: changed$hits"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning file=$tex::$msg"; else echo "$msg"; fi
  fi
done
[ "$stale" = 1 ] || echo "docs: no section's sources changed without it"
