-- Controller-only scenarios share the published $06E0 ABI and bounded waits.
local h = {}
local out_file = assert(os.getenv('NESSY_TEST_OUT'))
local artifact_dir = assert(os.getenv('NESSY_ARTIFACT_DIR'))
local prg = manager.machine.devices[':maincpu'].spaces['program']
local screen = manager.machine.screens[':screen']
local pads = {}
local rows = {'label,frame,p1x,p1y,p1damage,p1kos,p2x,p2y,p2damage,p2kos,p1jab,p1special,p1stun,p2jab,p2special,p2stun'}

for player = 1, 2 do
  pads[player] = {}
  local prefix = 'P' .. player .. ' '
  local port = manager.machine.ioport.ports[':ctrl' .. player .. ':joypad:JOYPAD']
  for _, field in pairs(port.fields) do
    local name = tostring(field.name)
    if name:sub(1, #prefix) == prefix and not pads[player][name:sub(#prefix + 1)] then
      pads[player][name:sub(#prefix + 1)] = field
    end
  end
end

function h.rb(offset)
  return prg:read_u8(0x06E0 + offset)
end

function h.rs16(offset)
  local value = h.rb(offset) + h.rb(offset + 1) * 256
  return value >= 32768 and value - 65536 or value
end

function h.trace(label)
  table.insert(rows, string.format('%s,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d',
    label or '', h.rb(4), h.rs16(8), h.rs16(10), h.rb(12), h.rb(13),
    h.rs16(14), h.rs16(16), h.rb(18), h.rb(19),
    h.rb(24), h.rb(25), h.rb(26), h.rb(27), h.rb(28), h.rb(29)))
end

function h.pad(player, buttons)
  for _, field in pairs(pads[player]) do field:set_value(0) end
  for _, button in ipairs(buttons or {}) do
    assert(pads[player][button], 'unknown button ' .. button):set_value(1)
  end
end

function h.clear()
  h.pad(1)
  h.pad(2)
end

function h.tick(count, label)
  for _ = 1, count or 1 do
    local previous = h.rb(4)
    local advanced = false
    for _ = 1, 152 do
      emu.wait_next_frame()
      if h.rb(4) ~= previous then advanced = true; break end
    end
    assert(advanced, 'gameplay frame counter stopped advancing')
    h.trace(label)
  end
end

function h.until_true(predicate, limit, label)
  for _ = 1, limit do
    if predicate() then return true end
    h.tick(1, label)
  end
  return predicate()
end

function h.tap(player, button, frames)
  h.pad(player, {button})
  h.tick(frames or 2)
  h.pad(player)
  h.tick(2)
end

function h.snapshot(name)
  screen:snapshot(artifact_dir .. '/' .. name .. '.png')
end

function h.start(mode)
  h.clear()
  if h.rb(2) == 2 then
    h.pad(1, {'Start', 'Select'})
    assert(h.until_true(function() return h.rb(2) == 0 end, 20), 'failed to return to title')
    h.clear()
    h.tick(3)
  end
  assert(h.rb(2) == 0, 'expected title')
  for _ = 1, mode or 0 do h.tap(1, 'Down') end
  h.tap(1, 'Start')
  assert(h.rb(2) == 1, 'failed to reach lobby')
end

function h.battle()
  h.tap(1, 'Start')
  assert(h.until_true(function() return h.rb(2) == 2 end, 20), 'failed to enter battle')
end

function h.run(body)
  for _ = 1, 120 do emu.wait_next_frame() end
  local ok, message = pcall(function()
    assert(h.rb(0) == 0x53 and h.rb(1) == 0x4e, 'runtime signature missing')
    return body()
  end)
  h.clear()
  local report = assert(io.open(out_file, 'w'))
  report:write((ok and 'PASS' or 'FAIL') .. '\n' .. tostring(message) .. '\n')
  report:close()
  local trace = assert(io.open(artifact_dir .. '/trace.csv', 'w'))
  trace:write(table.concat(rows, '\n') .. '\n')
  trace:close()
  manager.machine:exit()
end

return h
