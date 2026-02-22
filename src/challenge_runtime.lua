-- nvimkata challenge runtime
-- Variables injected by Rust preamble:
--   _VK_NUMBER, _VK_TITLE, _VK_PAR, _VK_HINT, _VK_DETAILED_HINT,
--   _VK_FREESTYLE, _VK_RESULTS_PATH, _VK_TARGET_PATH, _VK_START_PATH,
--   _VK_THRESHOLD_A, _VK_THRESHOLD_B, _VK_THRESHOLD_C, _VK_THRESHOLD_D,
--   _VK_THRESHOLD_E, _VK_THRESHOLD_F

local ks = 0
local done = false
local cmd_start_ks = nil
local win = vim.api.nvim_get_current_win()
local buf = vim.api.nvim_get_current_buf()
local t0 = vim.uv.now()
local showing_hint = false
local f1_code = vim.api.nvim_replace_termcodes("<F1>", true, false, true)
local key_log = {}
local timer_tick

local function norm(lines)
  local r = {}
  for _, l in ipairs(lines) do
    r[#r + 1] = l:gsub("%s+$", "")
  end
  while #r > 0 and r[#r] == "" do
    r[#r] = nil
  end
  return table.concat(r, "\n")
end

local target_norm = norm(vim.fn.readfile(_VK_TARGET_PATH))

local function set_bar(n, elapsed)
  if not vim.api.nvim_win_is_valid(win) then
    return
  end
  local m = math.floor(elapsed / 60)
  local s = elapsed % 60
  local bar = string.format("  #%03d - %s | %d keys | %02d:%02d", _VK_NUMBER, _VK_TITLE, n, m, s)
  if _VK_FREESTYLE then
    bar = bar .. " | FREESTYLE"
  end
  vim.api.nvim_set_option_value("winbar", bar:gsub("%%", "%%%%"), { win = win })
end

local function show_hint_float(title, text, footer)
  showing_hint = true
  local ui = vim.api.nvim_list_uis()[1] or { width = 80, height = 24 }
  local max_width = math.min(60, ui.width - 4)

  -- Word-wrap text into lines
  local lines = {}
  for _, paragraph in ipairs(vim.split(text, "\n")) do
    local line = ""
    for word in paragraph:gmatch("%S+") do
      if #line + #word + 1 > max_width - 4 then
        lines[#lines + 1] = "  " .. line
        line = word
      elseif #line > 0 then
        line = line .. " " .. word
      else
        line = word
      end
    end
    if #line > 0 then
      lines[#lines + 1] = "  " .. line
    end
  end

  table.insert(lines, 1, "")
  table.insert(lines, "")
  table.insert(lines, "  " .. footer)
  table.insert(lines, "")

  local width = max_width
  for _, l in ipairs(lines) do
    if #l + 2 > width then width = #l + 2 end
  end

  local float_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(float_buf, 0, -1, false, lines)
  vim.api.nvim_set_option_value("modifiable", false, { buf = float_buf })
  vim.api.nvim_set_option_value("bufhidden", "wipe", { buf = float_buf })

  local row = math.floor((ui.height - #lines) / 2)
  local col = math.floor((ui.width - width) / 2)

  local float_win = vim.api.nvim_open_win(float_buf, true, {
    relative = "editor",
    row = row,
    col = col,
    width = width,
    height = #lines,
    style = "minimal",
    border = "rounded",
    title = " " .. title .. " ",
    title_pos = "center",
  })

  vim.cmd("redraw")
  local ok, key = pcall(vim.fn.getcharstr)

  if vim.api.nvim_win_is_valid(float_win) then
    vim.api.nvim_win_close(float_win, true)
  end
  showing_hint = false

  -- Return whether F1 was pressed to dismiss
  if ok and key == f1_code then
    return true
  end
  return false
end

local function write_results(n, elapsed, keys)
  local f = io.open(_VK_RESULTS_PATH, "w")
  if f then
    f:write(tostring(n) .. "\n" .. tostring(elapsed) .. "\n" .. keys)
    f:close()
  end
end

local function get_grade(n)
  if _VK_FREESTYLE then
    return nil
  end
  if n <= _VK_THRESHOLD_A then
    return "GRADE A"
  elseif n <= _VK_THRESHOLD_B then
    return "GRADE B"
  elseif n <= _VK_THRESHOLD_C then
    return "GRADE C"
  elseif n <= _VK_THRESHOLD_D then
    return "GRADE D"
  elseif n <= _VK_THRESHOLD_E then
    return "GRADE E"
  else
    return "GRADE F"
  end
end

local function show_result_float(n, elapsed, matched)
  local grade = matched and get_grade(n) or nil
  local m = math.floor(elapsed / 60)
  local s = elapsed % 60

  local lines = {}
  table.insert(lines, "")
  if _VK_FREESTYLE then
    if matched then
      table.insert(lines, "  COMPLETED")
    else
      table.insert(lines, "  FAILED")
    end
  elseif grade then
    table.insert(lines, "  " .. grade)
  else
    table.insert(lines, "  FAILED")
  end
  table.insert(lines, "")
  if _VK_FREESTYLE then
    table.insert(lines, string.format("  %d keys | %02d:%02d", n, m, s))
  else
    table.insert(lines, string.format("  %d keys (par: %d) | %02d:%02d", n, _VK_PAR, m, s))
  end
  table.insert(lines, "")
  table.insert(lines, "  r: retry | any other key: exit")
  table.insert(lines, "")

  local width = 40
  for _, line in ipairs(lines) do
    if #line + 2 > width then
      width = #line + 2
    end
  end
  local height = #lines

  local ui = vim.api.nvim_list_uis()[1] or { width = 80, height = 24 }
  local row = math.floor((ui.height - height) / 2)
  local col = math.floor((ui.width - width) / 2)

  local float_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(float_buf, 0, -1, false, lines)

  local float_win = vim.api.nvim_open_win(float_buf, true, {
    relative = "editor",
    row = row,
    col = col,
    width = width,
    height = height,
    style = "minimal",
    border = "rounded",
    title = " Result ",
    title_pos = "center",
  })

  vim.api.nvim_set_option_value("modifiable", false, { buf = float_buf })
  vim.api.nvim_set_option_value("bufhidden", "wipe", { buf = float_buf })

  -- Highlight grade/fail line
  if grade then
    vim.api.nvim_buf_add_highlight(float_buf, -1, "DiagnosticOk", 1, 0, -1)
  else
    vim.api.nvim_buf_add_highlight(float_buf, -1, "DiagnosticError", 1, 0, -1)
  end

  -- Wait for keypress
  local ok, char = pcall(vim.fn.getcharstr)
  if not ok then
    char = ""
  end

  -- Close float
  if vim.api.nvim_win_is_valid(float_win) then
    vim.api.nvim_win_close(float_win, true)
  end

  if char == "r" then
    return true
  end
  return false
end

local function do_retry()
  -- Reset buffer from start file
  local start_lines = vim.fn.readfile(_VK_START_PATH)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, start_lines)
  -- Reset state
  ks = 0
  done = false
  cmd_start_ks = nil
  t0 = vim.uv.now()
  key_log = {}
  set_bar(0, 0)
  -- Restart timer
  _G._ks_timer:start(
    100,
    100,
    vim.schedule_wrap(function()
      timer_tick()
    end)
  )
end

local function finish(n, elapsed, keys, matched)
  done = true
  _G._ks_timer:stop()
  write_results(n, elapsed, keys)

  local retry = show_result_float(n, elapsed, matched)
  if retry then
    do_retry()
  else
    vim.cmd("silent! write | qall!")
  end
end

set_bar(0, 0)

-- F1 hint popup (filtered from keystroke count)
for _, mode in ipairs({ "n", "i", "v" }) do
  vim.keymap.set(mode, "<F1>", function()
    local hint_footer = _VK_DETAILED_HINT ~= "" and "F1: detailed hint | any key: close" or "any key: close"
    local dismissed_with_f1 = show_hint_float("Hint", _VK_HINT, hint_footer)
    if dismissed_with_f1 and _VK_DETAILED_HINT ~= "" then
      show_hint_float("Detailed Hint", _VK_DETAILED_HINT, "any key: close")
    end
  end, { noremap = true, silent = true })
end

-- Track command-line entry for :w subtraction
vim.api.nvim_create_autocmd("CmdlineEnter", {
  callback = function()
    cmd_start_ks = ks
  end,
})
vim.api.nvim_create_autocmd("CmdlineLeave", {
  callback = function()
    cmd_start_ks = nil
  end,
})

-- Count keystrokes (filter F1)
vim.on_key(function(_, typed)
  if done or showing_hint or not typed or typed == "" then
    return
  end
  if typed == f1_code then
    return
  end
  ks = ks + 1
  key_log[#key_log + 1] = vim.fn.keytrans(typed)
  local elapsed = math.floor((vim.uv.now() - t0) / 1000)
  set_bar(ks, elapsed)
end)

-- Timer tick function
timer_tick = function()
  if done then
    return
  end
  local elapsed = math.floor((vim.uv.now() - t0) / 1000)
  set_bar(ks, elapsed)
  local matched = norm(vim.api.nvim_buf_get_lines(buf, 0, -1, false)) == target_norm
  if matched then
    finish(ks, elapsed, table.concat(key_log), matched)
  end
end

local t = vim.uv.new_timer()
_G._ks_timer = t
_G._ks_stop = function()
  if done then
    return
  end
  done = true
  t:stop()
  local elapsed = math.floor((vim.uv.now() - t0) / 1000)
  local save_ks = 2
  if cmd_start_ks then
    save_ks = ks - cmd_start_ks + 1
  end
  local final_ks = math.max(0, ks - save_ks)
  write_results(final_ks, elapsed, table.concat(key_log, "", 1, final_ks))
end

t:start(
  100,
  100,
  vim.schedule_wrap(function()
    timer_tick()
  end)
)
