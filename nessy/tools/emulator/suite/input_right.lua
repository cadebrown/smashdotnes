-- Moves through TITLE->LOBBY->BATTLE and verifies P1 X increases with RIGHT.

local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/input_right.txt'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local BASE = 0x06E0

local function rb(off)
  return prg:read_u8(BASE + off)
end

local function rs16(off)
  local lo = rb(off)
  local hi = rb(off + 1)
  local v = lo + hi * 256
  if v >= 32768 then
    v = v - 65536
  end
  return v
end

local p1_port = manager.machine.ioport.ports[':ctrl1:joypad:JOYPAD']
local p1 = {}
for _, field in pairs(p1_port.fields) do
  local name = tostring(field.name)
  if string.sub(name, 1, 3) == 'P1 ' and p1[name] == nil then
    p1[name] = field
  end
end

local function set_p1(button, down)
  local key = 'P1 ' .. button
  local f = p1[key]
  if f ~= nil then
    f:set_value(down and 1 or 0)
  end
end

local function clear_p1()
  for _, field in pairs(p1) do
    field:set_value(0)
  end
end

local function wait_frames(n)
  for _ = 1, n do
    emu.wait_next_frame()
  end
end

local function wait_game_frames(n)
  local previous = rb(4)
  local advanced = 0
  local polls = 0
  local poll_limit = n * 32 + 120
  while advanced < n and polls < poll_limit do
    wait_frames(1)
    polls = polls + 1
    local current = rb(4)
    if current ~= previous then
      advanced = advanced + 1
      previous = current
    end
  end
  return advanced == n
end

local function tap_p1(button, hold_frames)
  clear_p1()
  set_p1(button, true)
  wait_game_frames(hold_frames or 2)
  clear_p1()
  wait_game_frames(2)
end

local function write_result(ok, message)
  local f = io.open(out_file, 'w')
  if f then
    f:write((ok and 'PASS' or 'FAIL') .. '\n')
    f:write(message .. '\n')
    f:close()
  end
end

wait_frames(120)
if rb(2) ~= 0 then
  write_result(false, 'expected TITLE state after boot')
  manager.machine:exit()
  return
end

tap_p1('Start', 3)
wait_game_frames(3)
if rb(2) ~= 1 then
  write_result(false, 'expected LOBBY state after first START')
  manager.machine:exit()
  return
end

tap_p1('Start', 3)
wait_game_frames(3)
if rb(2) ~= 2 then
  write_result(false, 'expected BATTLE state after second START')
  manager.machine:exit()
  return
end

local x_before = rs16(8)
clear_p1()
set_p1('Right', true)
wait_game_frames(20)
clear_p1()
wait_game_frames(2)
local x_after = rs16(8)

if x_after <= x_before then
  write_result(false, string.format('expected X to increase: before=%d after=%d', x_before, x_after))
  manager.machine:exit()
  return
end

write_result(true, string.format('right input moved X: before=%d after=%d', x_before, x_after))
manager.machine:exit()
