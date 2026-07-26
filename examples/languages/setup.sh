#!/usr/bin/env bash
# Stage the parser shared objects and highlights queries this example needs.
#
# Grammars are external compiled parsers + queries. The copies used by the
# deployed book are committed for reproducible cloud builds; this script
# refreshes them from a local nvim-treesitter install.
# Most languages are copied directly from that install.
# Macaulay2 is built from its grammar repo so the parser and query stay
# version-consistent (the query uses node patterns the matching parser provides).
#   NVIM_TS_DIR   nvim-treesitter dir (default: ~/.local/share/nvim/lazy/nvim-treesitter)
#   M2_GRAMMAR    tree-sitter-macaulay2 repo (default: ~/m2/tree-sitter-macaulay2)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
ts="${NVIM_TS_DIR:-$HOME/.local/share/nvim/lazy/nvim-treesitter}"
m2="${M2_GRAMMAR:-$HOME/m2/tree-sitter-macaulay2}"

mkdir -p "$here/parsers"

# Copy a compiled parser out of the nvim-treesitter install.
parser() { # <parser-file-stem>
  local src="$ts/parser/$1.so"
  [[ -f "$src" ]] || { echo "missing parser: $src (try: nvim '+TSInstallSync $1' +qa)" >&2; exit 1; }
  cp "$src" "$here/parsers/$1.so"
}

# Copy a highlights query out of the nvim-treesitter install. nvim resolves
# `; inherits:` modelines itself; mdbook-tsitter receives one query file, so
# flatten the small inheritance chains used by these example languages.
query() { # <lang>
  local lang="$1"
  local parts=()
  case "$lang" in
    cpp) parts=(c) ;;
    javascript) parts=(ecma jsx) ;;
    php) parts=(php_only) ;;
    typescript) parts=(ecma) ;;
  esac
  parts+=("$lang")

  mkdir -p "$here/queries/$lang"
  for part in "${parts[@]}"; do
    local src="$ts/queries/$part/highlights.scm"
    [[ -f "$src" ]] || {
      echo "missing query: $src" >&2
      exit 1
    }
  done

  {
    for index in "${!parts[@]}"; do
      local part="${parts[$index]}"
      sed '/^; inherits:/d' "$ts/queries/$part/highlights.scm"
      if ((index + 1 < ${#parts[@]})); then
        printf '\n'
      fi
    done
  } > "$here/queries/$lang/highlights.scm"
  perl -0pi -e 's/\n+\z/\n/' \
    "$here/queries/$lang/highlights.scm"
}

for lang in \
  bash c cpp go haskell java javascript lua markdown php python rust typescript
do
  parser "$lang"
  query "$lang"
done

# Macaulay2: compile the parser from its grammar repo and use that repo's query.
[[ -f "$m2/src/parser.c" ]] || { echo "missing m2 grammar repo: $m2/src/parser.c" >&2; exit 1; }
m2_scanner=(); [[ -f "$m2/src/scanner.c" ]] && m2_scanner=("$m2/src/scanner.c")
cc -O2 -shared -fPIC -I "$m2/src" "$m2/src/parser.c" "${m2_scanner[@]}" -o "$here/parsers/macaulay2.so"
mkdir -p "$here/queries/macaulay2"
cp "$m2/queries/macaulay2/highlights.scm" "$here/queries/macaulay2/highlights.scm"

# A standard-form injections query: highlight each fenced block inside a
# Markdown block with the grammar named by its info string. tree-sitter-highlight
# reads @injection.language / @injection.content directly (unlike nvim's custom
# #set-lang-from-info-string! directive).
mkdir -p "$here/queries/markdown"
cat > "$here/queries/markdown/injections.scm" <<'SCM'
(fenced_code_block
  (info_string (language) @injection.language)
  (code_fence_content) @injection.content)
SCM

echo "staged parsers + queries into $here. Now run: mdbook build '$here'"
