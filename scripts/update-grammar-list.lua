-- Regenerate assets/grammars.json, the built-in list of known grammar
-- repositories, from nvim-treesitter's parser list at the tip of its main
-- branch. Each entry keeps the repository URL, the pinned revision, and the
-- grammar subdirectory when there is one. The list also records where
-- nvim-treesitter's own queries for these revisions live, used for grammars
-- that ship none.
--
-- Requires git and curl on PATH. Run with any Lua interpreter, on any OS:
--   lua scripts/update-grammar-list.lua
--   nvim -l scripts/update-grammar-list.lua

local repo = 'https://github.com/nvim-treesitter/nvim-treesitter'
local raw_root = 'https://raw.githubusercontent.com/nvim-treesitter/nvim-treesitter'
local list_path = 'lua/nvim-treesitter/parsers.lua'
local query_dir = 'runtime/queries'
local script_dir = arg[0]:match('^(.*)[/\\]') or '.'
local output = script_dir .. '/../assets/grammars.json'

--- The standard output of `command`, failing when it prints nothing.
local function capture(command)
  local pipe = assert(io.popen(command))
  local text = pipe:read('*a')
  pipe:close()
  if text == nil or text == '' then
    error('no output from: ' .. command)
  end
  return text
end

local commit = capture('git ls-remote ' .. repo .. ' refs/heads/main'):match('^(%x+)')
if not commit then
  error('could not resolve ' .. repo .. ' main')
end
local source = capture('curl -sSfL ' .. raw_root .. '/' .. commit .. '/' .. list_path)
local parsers = assert(load(source, list_path))()

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

-- Binary mode keeps the file's line endings `\n` on every platform.
local file = assert(io.open(output, 'wb'))
file:write(table.concat(lines, '\n'), '\n')
file:close()
print(string.format('wrote %d grammars from %s@%s to %s', #names, repo, commit, output))
