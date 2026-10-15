#!/usr/bin/env bash
# Regenerate assets/grammars.json, the built-in list of known grammar
# repositories, from nvim-treesitter's parser list at the tip of its main
# branch. Each entry keeps the repository URL, the pinned revision, and the
# grammar subdirectory when there is one. The list also records where
# nvim-treesitter's own queries for these revisions live, used for grammars
# that ship none.
#
# Requires git, curl and a Lua interpreter (lua, luajit or `nvim -l`).
#   LUA   Lua command to run (default: lua)
set -euo pipefail

repo="https://github.com/nvim-treesitter/nvim-treesitter"
list_path="lua/nvim-treesitter/parsers.lua"
query_dir="runtime/queries"
here="$(cd "$(dirname "$0")" && pwd)"
output="$here/../assets/grammars.json"

commit="$(git ls-remote "$repo" refs/heads/main | cut -f1)"
[[ -n "$commit" ]] || { echo "could not resolve $repo main" >&2; exit 1; }

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
curl -sSfL -o "$workdir/parsers.lua" \
  "https://raw.githubusercontent.com/nvim-treesitter/nvim-treesitter/$commit/$list_path"

cat > "$workdir/convert.lua" <<'LUA'
local parsers = dofile(arg[1])
local repo, commit, query_dir = arg[2], arg[3], arg[4]

local function quote(text)
  return '"' .. text:gsub('[%c"\\]', function(c)
    return string.format('\\u%04x', c:byte())
  end) .. '"'
end

local names = {}
for name, parser in pairs(parsers) do
  if parser.install_info and parser.install_info.url then
    names[#names + 1] = name
  end
end
table.sort(names)

local lines = {
  '{',
  '  "queries": { "url": ' .. quote(repo) .. ', "revision": ' .. quote(commit)
    .. ', "directory": ' .. quote(query_dir) .. ' },',
  '  "grammars": {',
}
for index, name in ipairs(names) do
  local info = parsers[name].install_info
  local fields = { '"url": ' .. quote(info.url), '"revision": ' .. quote(info.revision) }
  if info.location then
    fields[#fields + 1] = '"location": ' .. quote(info.location)
  end
  local separator = index < #names and ',' or ''
  lines[#lines + 1] = '    ' .. quote(name) .. ': { ' .. table.concat(fields, ', ') .. ' }' .. separator
end
lines[#lines + 1] = '  }'
lines[#lines + 1] = '}'
print(table.concat(lines, '\n'))
LUA

${LUA:-lua} "$workdir/convert.lua" "$workdir/parsers.lua" "$repo" "$commit" "$query_dir" > "$output"
echo "wrote $(grep -c '^    "' "$output") grammars from $repo@$commit to $output"
