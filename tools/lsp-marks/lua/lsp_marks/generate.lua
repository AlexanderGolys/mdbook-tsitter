--- Writes the semantic tokens of a language server as `§type.modifier|text§`
--- marks.
local parse = require("lsp_marks.parse")

local M = {}

-- Tokens a tree-sitter grammar already colours; marking them only adds noise.
-- A keyword is still kept when it carries a modifier, as rust-analyzer's
-- `keyword.controlFlow` does.
local LEXICAL_TYPES = {
  comment = true, string = true, number = true, operator = true, punctuation = true,
  bracket = true, parenthesis = true, brace = true, angle = true, semicolon = true,
  comma = true, colon = true, dot = true, boolean = true, escapeSequence = true,
  formatSpecifier = true, character = true, keyword = true, attributeBracket = true,
}
local KEPT_WITH_MODIFIERS = { keyword = true }

-- Modifiers that say little a reader can use, left out of the marks.
local DEFAULT_SKIPPED_MODIFIERS = {
  public = true, reference = true, library = true, trait = true, associated = true,
  attribute = true, macro = true, procMacro = true,
}

local REQUEST_TIMEOUT_MS = 20000

---@class lsp_marks.GenerateOptions
---@field skip? table<string, boolean> token types to leave out
---@field keep? table<string, boolean> token types to keep although skipped by default
---@field modifiers? table<string, boolean> the only modifiers to write

local function modifier_names(bits, legend)
  local names = {}
  for index, name in ipairs(legend.tokenModifiers) do
    if bit.band(bits, bit.lshift(1, index - 1)) ~= 0 then names[#names + 1] = name end
  end
  return names
end

-- The tokens of each line, from the protocol's delta encoding. Columns are
-- UTF-16 code units, turned here into byte offsets.
local function decode(data, legend, lines)
  local by_line = {}
  local line, column = 0, 0
  for index = 1, #data, 5 do
    local delta_line, delta_column, length, type_index, bits = unpack(data, index, index + 4)
    line = line + delta_line
    column = delta_line == 0 and column + delta_column or delta_column
    local text = lines[line + 1]
    if text then
      by_line[line + 1] = by_line[line + 1] or {}
      table.insert(by_line[line + 1], {
        start = vim.str_byteindex(text, "utf-16", column),
        stop = vim.str_byteindex(text, "utf-16", column + length),
        type = legend.tokenTypes[type_index + 1],
        modifiers = modifier_names(bits, legend),
      })
    end
  end
  return by_line
end

local function written_modifiers(names, only)
  return vim.tbl_filter(function(name)
    if only then return only[name] end
    return not DEFAULT_SKIPPED_MODIFIERS[name]
  end, names)
end

local function wanted(token_type, modifiers, text, options)
  if options.skip[token_type] or text:match("^%s*$") then return false end
  if options.keep[token_type] or not LEXICAL_TYPES[token_type] then return true end
  return KEPT_WITH_MODIFIERS[token_type] and #modifiers > 0
end

local function escaped(text)
  return (text:gsub(parse.MARK, parse.MARK .. parse.MARK))
end

local function marked_line(text, tokens, options)
  local pieces, position = {}, 0
  for _, token in ipairs(tokens or {}) do
    local modifiers = written_modifiers(token.modifiers, options.modifiers)
    local marked = text:sub(token.start + 1, token.stop)
    -- A mark right after another would read as an escaped marker.
    if wanted(token.type, modifiers, marked, options) and token.start > position then
      local name = token.type .. table.concat(vim.tbl_map(function(modifier)
        return parse.MODIFIER_SEPARATOR .. modifier
      end, modifiers))
      pieces[#pieces + 1] = escaped(text:sub(position + 1, token.start))
      pieces[#pieces + 1] = parse.MARK .. name .. parse.TOKEN_SEPARATOR .. escaped(marked) .. parse.MARK
      position = token.stop
    end
  end
  pieces[#pieces + 1] = escaped(text:sub(position + 1))
  return table.concat(pieces)
end

---@param client vim.lsp.Client
---@param buffer integer
---@return integer[]|nil
function M.request_tokens(client, buffer)
  local response = client:request_sync(
    "textDocument/semanticTokens/full",
    { textDocument = vim.lsp.util.make_text_document_params(buffer) },
    REQUEST_TIMEOUT_MS,
    buffer
  )
  return response and response.result and response.result.data
end

---@param buffer integer
---@param first integer 1-based first line
---@param last integer 1-based last line, inclusive
---@param options? lsp_marks.GenerateOptions
---@param data? integer[] tokens already requested
---@return string[]|nil lines
---@return string|nil error
function M.lines(buffer, first, last, options, data)
  options = vim.tbl_extend("keep", options or {}, { skip = {}, keep = {} })
  local client = vim.lsp.get_clients({ bufnr = buffer, method = "textDocument/semanticTokens/full" })[1]
  if not client then return nil, "no language server with semantic tokens is attached to this buffer" end
  data = data or M.request_tokens(client, buffer)
  if not data or #data == 0 then return nil, "the server sent no semantic tokens; is it still loading?" end

  local lines = vim.api.nvim_buf_get_lines(buffer, 0, -1, false)
  local by_line = decode(data, client.server_capabilities.semanticTokensProvider.legend, lines)
  local marked = {}
  for number = first, last do
    marked[#marked + 1] = marked_line(lines[number], by_line[number], options)
  end
  return marked
end

return M
