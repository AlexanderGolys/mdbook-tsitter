--- The `§type.modifier|text§` syntax of mdbook-tsitter code blocks: `§` opens
--- and closes a token, `§§` stands for a literal `§`.
local M = {}

M.MARK = "§"
M.TOKEN_SEPARATOR = "|"
M.MODIFIER_SEPARATOR = "."

---@class lsp_marks.Mark
---@field start integer byte offset in the stripped source, 0-based
---@field stop integer byte offset just past the marked text
---@field type string
---@field modifiers string[]

---@class lsp_marks.Marked
---@field source string the text without markers
---@field marks lsp_marks.Mark[]

local function parse_token_text(token_text)
  local separator = token_text:find(M.TOKEN_SEPARATOR, 1, true)
  if not separator then
    return nil, ("`%s%s%s` needs a token before `%s`"):format(M.MARK, token_text, M.MARK, M.TOKEN_SEPARATOR)
  end
  local token, text = token_text:sub(1, separator - 1), token_text:sub(separator + 1)
  local names = vim.split(token, M.MODIFIER_SEPARATOR, { plain = true })
  for _, name in ipairs(names) do
    if not name:match("^[%w_]+$") then
      return nil, ("`%s` is not a token: expected a type and modifiers separated by `%s`"):format(
        token, M.MODIFIER_SEPARATOR
      )
    end
  end
  return { type = table.remove(names, 1), modifiers = names, text = text }
end

---@param text string
---@return lsp_marks.Marked|nil marked
---@return string|nil error
function M.parse(text)
  local source, marks = {}, {}
  local size = 0
  local token_text = nil
  local position = 1
  while true do
    local at = text:find(M.MARK, position, true)
    local chunk = text:sub(position, (at or #text + 1) - 1)
    if token_text then
      token_text[#token_text + 1] = chunk
    else
      source[#source + 1] = chunk
      size = size + #chunk
    end
    if not at then break end

    local after = at + #M.MARK
    if text:sub(after, after + #M.MARK - 1) == M.MARK then
      local target = token_text or source
      target[#target + 1] = M.MARK
      if not token_text then size = size + #M.MARK end
      position = after + #M.MARK
    elseif token_text then
      local token, err = parse_token_text(table.concat(token_text))
      if not token then return nil, err end
      marks[#marks + 1] = { start = size, stop = size + #token.text, type = token.type, modifiers = token.modifiers }
      source[#source + 1] = token.text
      size = size + #token.text
      token_text = nil
      position = after
    else
      token_text = {}
      position = after
    end
  end
  if token_text then
    return nil, ("`%s%s` is never closed; write `%s%s` for a literal `%s`"):format(
      M.MARK, table.concat(token_text), M.MARK, M.MARK, M.MARK
    )
  end
  return { source = table.concat(source), marks = marks }
end

return M
