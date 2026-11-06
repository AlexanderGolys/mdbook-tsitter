-- :LspMarksYank   copies the semantic tokens of the language server attached to
--                 this buffer, over a range, as `§type|text§` marks.
-- :LspMarksPreview [filetype]
--                 shows this buffer, written in marks, as highlighted code in a
--                 split that follows every edit; run it again to close it.

if vim.g.loaded_lsp_marks then return end
vim.g.loaded_lsp_marks = true

vim.api.nvim_create_user_command("LspMarksYank", function(command)
  local lines, err = require("lsp_marks.generate").lines(0, command.line1, command.line2)
  if not lines then return vim.notify("lsp-marks: " .. err, vim.log.levels.ERROR) end
  local text = table.concat(lines, "\n")
  vim.fn.setreg('"', text, "l")
  pcall(vim.fn.setreg, "+", text, "l")
  vim.notify(("lsp-marks: yanked %d lines"):format(#lines))
end, { range = "%", desc = "Yank a range as semantic-token marks" })

vim.api.nvim_create_user_command("LspMarksPreview", function(command)
  local filetype = command.args ~= "" and command.args or nil
  require("lsp_marks.preview").toggle(vim.api.nvim_get_current_buf(), filetype)
end, { nargs = "?", desc = "Preview a buffer of semantic-token marks as highlighted code" })
