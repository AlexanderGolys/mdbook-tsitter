--- A split showing a buffer of `§type.modifier|text§` marks as highlighted code:
--- the markers are stripped so tree-sitter parses clean code, and each token is
--- coloured with the highlight groups Neovim uses for semantic tokens
--- (`@lsp.type.*`, `@lsp.mod.*`, `@lsp.typemod.*`).
local parse = require("lsp_marks.parse")

local M = {}

local namespace = vim.api.nvim_create_namespace("lsp_marks")
local previews = {} ---@type table<integer, integer> source buffer -> preview buffer

-- Neovim's own priorities for the three groups of a token, in order.
local function priorities()
  local base = (vim.hl or vim.highlight).priorities.semantic_tokens
  return { type = base, modifier = base + 1, type_modifier = base + 2 }
end

local function highlight_groups(mark, filetype)
  local priority = priorities()
  local suffix = filetype ~= "" and "." .. filetype or ""
  local groups = { { "@lsp.type." .. mark.type .. suffix, priority.type } }
  for _, modifier in ipairs(mark.modifiers) do
    groups[#groups + 1] = { "@lsp.mod." .. modifier .. suffix, priority.modifier }
    groups[#groups + 1] = { "@lsp.typemod." .. mark.type .. "." .. modifier .. suffix, priority.type_modifier }
  end
  return groups
end

-- Byte offset in the whole text -> 0-based row and column.
local function locator(lines)
  local starts, offset = {}, 0
  for index, line in ipairs(lines) do
    starts[index] = offset
    offset = offset + #line + 1
  end
  return function(position)
    local low, high = 1, #starts
    while low < high do
      local middle = math.ceil((low + high) / 2)
      if starts[middle] <= position then low = middle else high = middle - 1 end
    end
    return low - 1, position - starts[low]
  end
end

local function render(source, target, filetype)
  local text = table.concat(vim.api.nvim_buf_get_lines(source, 0, -1, false), "\n")
  local marked, err = parse.parse(text)
  if not marked then
    vim.notify("lsp-marks: " .. err, vim.log.levels.WARN)
    return
  end
  local lines = vim.split(marked.source, "\n", { plain = true })
  vim.api.nvim_buf_set_lines(target, 0, -1, false, lines)
  if vim.bo[target].filetype ~= filetype then vim.bo[target].filetype = filetype end

  vim.api.nvim_buf_clear_namespace(target, namespace, 0, -1)
  local locate = locator(lines)
  for _, mark in ipairs(marked.marks) do
    local start_row, start_col = locate(mark.start)
    local end_row, end_col = locate(mark.stop)
    for _, group in ipairs(highlight_groups(mark, filetype)) do
      vim.api.nvim_buf_set_extmark(target, namespace, start_row, start_col, {
        end_row = end_row, end_col = end_col, hl_group = group[1], priority = group[2], strict = false,
      })
    end
  end
end

---@param source integer
---@param filetype? string defaults to the file type of `source`
function M.open(source, filetype)
  filetype = filetype or vim.bo[source].filetype
  if previews[source] and vim.api.nvim_buf_is_valid(previews[source]) then
    M.close(source)
  end
  local target = vim.api.nvim_create_buf(false, true)
  vim.bo[target].bufhidden = "wipe"
  vim.bo[target].swapfile = false
  vim.api.nvim_buf_set_name(target, ("lsp-marks://%d"):format(source))
  previews[source] = target

  local origin = vim.api.nvim_get_current_win()
  vim.cmd("vsplit")
  vim.api.nvim_win_set_buf(0, target)
  vim.api.nvim_set_current_win(origin)

  render(source, target, filetype)
  local group = vim.api.nvim_create_augroup("lsp_marks_" .. source, { clear = true })
  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = group,
    buffer = source,
    callback = function()
      if vim.api.nvim_buf_is_valid(target) then render(source, target, filetype) end
    end,
  })
  vim.api.nvim_create_autocmd("BufWipeout", {
    group = group,
    buffer = target,
    callback = function() previews[source] = nil; pcall(vim.api.nvim_del_augroup_by_id, group) end,
  })
end

---@param source integer
function M.close(source)
  local target = previews[source]
  if target and vim.api.nvim_buf_is_valid(target) then
    vim.api.nvim_buf_delete(target, { force = true })
  end
  previews[source] = nil
end

---@param source integer
function M.toggle(source, filetype)
  if previews[source] and vim.api.nvim_buf_is_valid(previews[source]) then
    M.close(source)
  else
    M.open(source, filetype)
  end
end

return M
