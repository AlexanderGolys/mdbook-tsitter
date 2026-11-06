-- Prints a line range of a source file with the semantic tokens of its language
-- server written as `§type.modifier|text§` marks, ready for a block annotated
-- `lsp` in an mdbook-tsitter book. It starts the server itself, as a bare LSP
-- client, so it needs no editor configuration and works with any server. To
-- use the server of a running editor, with its own settings, use :LspMarksYank.
--
--   nvim --headless -l tools/lsp-marks/headless.lua <file> [first:last] [options]
--
--   first:last        1-based, inclusive line range; the whole file by default
--   --server=<cmd>    server command line; chosen from the file type by default
--   --root=<dir>      workspace root; the nearest ancestor holding a root marker
--   --skip=<a,b,...>  token types to leave out, on top of the default ones
--   --keep=<a,b,...>  token types to keep although they are skipped by default
--   --modifiers=<a,b> modifiers to write instead of all but the uninformative ones
--   --timeout=<s>     how long to wait for the server to finish loading

local script_directory = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":p:h")
vim.opt.runtimepath:prepend(script_directory)
local generate = require("lsp_marks.generate")

-- Servers by file type, and what marks a workspace root for them.
local SERVERS = {
  rust = { cmd = { "rust-analyzer" }, root_markers = { "Cargo.toml" } },
  lua = { cmd = { "lua-language-server" }, root_markers = { ".luarc.json", ".git" } },
  python = { cmd = { "basedpyright-langserver", "--stdio" }, root_markers = { "pyproject.toml", ".git" } },
  typescript = { cmd = { "typescript-language-server", "--stdio" }, root_markers = { "tsconfig.json", "package.json" } },
  cpp = { cmd = { "clangd" }, root_markers = { "compile_commands.json", ".git" } },
  c = { cmd = { "clangd" }, root_markers = { "compile_commands.json", ".git" } },
  go = { cmd = { "gopls" }, root_markers = { "go.mod" } },
}

local POLL_SECONDS = 2
local DEFAULT_TIMEOUT_SECONDS = 240
local INITIALIZE_TIMEOUT_MS = 30000

local function fail(message)
  io.stderr:write("lsp-marks: " .. message .. "\n")
  os.exit(1)
end

local function parse_arguments(arguments)
  local parsed = { options = {} }
  for _, argument in ipairs(arguments) do
    local name, value = argument:match("^%-%-([%w-]+)=(.*)$")
    if name then
      parsed.options[name] = value
    elseif not parsed.file then
      parsed.file = argument
    else
      local first, last = argument:match("^(%d+):(%d+)$")
      if not first then fail("expected first:last, got `" .. argument .. "`") end
      parsed.first, parsed.last = tonumber(first), tonumber(last)
    end
  end
  if not parsed.file then
    fail("usage: nvim --headless -l tools/lsp-marks/headless.lua <file> [first:last] [options]")
  end
  return parsed
end

local function split_list(text)
  local set = {}
  for item in (text or ""):gmatch("[^,]+") do set[item] = true end
  return set
end

local function load_buffer(path)
  local buffer = vim.fn.bufadd(vim.fn.fnamemodify(path, ":p"))
  vim.fn.bufload(buffer)
  vim.bo[buffer].filetype = vim.filetype.match({ buf = buffer }) or ""
  return buffer
end

local function start_client(buffer, options)
  local filetype = vim.bo[buffer].filetype
  local server = SERVERS[filetype] or {}
  local cmd = options.server and vim.split(options.server, "%s+", { trimempty = true }) or server.cmd
  if not cmd then fail("no known server for file type `" .. filetype .. "`; pass --server=<cmd>") end
  local name = vim.api.nvim_buf_get_name(buffer)
  local root = options.root or vim.fs.root(name, server.root_markers or { ".git" }) or vim.fs.dirname(name)

  local state = { quiescent = false }
  local capabilities = vim.tbl_deep_extend("force", vim.lsp.protocol.make_client_capabilities(), {
    experimental = { serverStatusNotification = true },
  })
  local client_id = vim.lsp.start({
    name = cmd[1],
    cmd = cmd,
    root_dir = root,
    capabilities = capabilities,
    handlers = {
      -- rust-analyzer says when it has finished loading the workspace.
      ["experimental/serverStatus"] = function(_, status) state.quiescent = status.quiescent end,
    },
  }, { bufnr = buffer })
  if not client_id then fail("could not start `" .. cmd[1] .. "`") end
  local client = vim.lsp.get_client_by_id(client_id)
  -- Requests sent before the handshake ends make some servers quit.
  if not vim.wait(INITIALIZE_TIMEOUT_MS, function() return client.initialized end) then
    fail("`" .. cmd[1] .. "` did not finish starting")
  end
  return client, state
end

-- A server loads in the background, and answers with less until it has, so
-- ask until it reports itself idle and two answers in a row agree.
local function wait_for_tokens(client, state, buffer, timeout_seconds)
  local reports_status = client.name == "rust-analyzer"
  local deadline = vim.uv.hrtime() + timeout_seconds * 1e9
  local previous
  while vim.uv.hrtime() < deadline do
    local current = generate.request_tokens(client, buffer)
    local settled = (not reports_status or state.quiescent) and current and #current > 0
    if settled and previous and vim.deep_equal(previous, current) then return current end
    previous = current
    vim.wait(POLL_SECONDS * 1000)
  end
  fail("the server did not settle within " .. timeout_seconds .. " seconds; raise --timeout")
end

local parsed = parse_arguments(arg)
local buffer = load_buffer(parsed.file)
local client, state = start_client(buffer, parsed.options)
local data = wait_for_tokens(client, state, buffer, tonumber(parsed.options.timeout) or DEFAULT_TIMEOUT_SECONDS)

local line_count = vim.api.nvim_buf_line_count(buffer)
local lines, err = generate.lines(buffer, parsed.first or 1, parsed.last or line_count, {
  skip = split_list(parsed.options.skip),
  keep = split_list(parsed.options.keep),
  modifiers = parsed.options.modifiers and split_list(parsed.options.modifiers),
}, data)
if not lines then fail(err) end
io.stdout:write(table.concat(lines, "\n"), "\n")
client:stop()
vim.cmd("qa!")
