#!/usr/bin/env bash
# Stage the parser shared objects and highlights queries this example needs.
#
# Grammars are external (compiled parsers + third-party queries), so they are
# not committed. This script copies them out of a local nvim-treesitter install
# by default; override the source paths with the environment variables below.
# Everything it writes (parsers/, queries/) is gitignored.
#
#   NVIM_TS_DIR   nvim-treesitter dir (default: ~/.local/share/nvim/lazy/nvim-treesitter)
#   M2_QUERY      Macaulay2 highlights.scm (default: ~/m2/spectral-m2/queries/highlights.scm)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
ts="${NVIM_TS_DIR:-$HOME/.local/share/nvim/lazy/nvim-treesitter}"
m2_query="${M2_QUERY:-$HOME/m2/spectral-m2/queries/highlights.scm}"

mkdir -p "$here/parsers"

# Parser name in the nvim install -> fence tag used in book.toml.
parser() { # <parser-file-stem>
  local src="$ts/parser/$1.so"
  [[ -f "$src" ]] || { echo "missing parser: $src (try: nvim '+TSInstallSync $1' +qa)" >&2; exit 1; }
  cp "$src" "$here/parsers/$1.so"
}

query() { # <lang> <source-highlights.scm>
  [[ -f "$2" ]] || { echo "missing query: $2" >&2; exit 1; }
  mkdir -p "$here/queries/$1"
  cp "$2" "$here/queries/$1/highlights.scm"
}

for p in macaulay2 rust lua markdown haskell; do parser "$p"; done

for l in rust lua markdown haskell; do query "$l" "$ts/queries/$l/highlights.scm"; done
query macaulay2 "$m2_query"

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
