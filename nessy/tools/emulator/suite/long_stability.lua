-- Longer run stability check in battle state with signature retention and frame movement.

local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/long_stability.txt'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local BASE = 0x06E0

local function rb(off)
  return prg:read_u8(BASE + off)
end

local function wait_frames(n)
  for _ = 1, n do
    emu.wait_next_frame()
  end
end

local function wrapped_delta(before, after)
  local d = after - before
  if d < 0 then
    d = d + 256
  end
  return d
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
      advanced = advanced + wrapped_delta(previous, current)
      previous = current
    end
  end
  return advanced >= n
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

if rb(0) ~= 0x53 or rb(1) ~= 0x4E then
  write_result(false, 'missing SN signature at $06E0')
  manager.machine:exit()
  return
end

if rb(2) ~= 0 then
  write_result(false, 'expected TITLE state before transition')
  manager.machine:exit()
  return
end

tap_p1('Start', 3)
wait_game_frames(3)
tap_p1('Start', 3)
wait_game_frames(3)

if rb(2) ~= 2 then
  write_result(false, 'expected BATTLE state before long run')
  manager.machine:exit()
  return
end

local start_frame = rb(4)
local previous_frame = start_frame
local advanced = 0
local polls = 0
while advanced < 240 and polls < 4800 do
  clear_p1()
  if (advanced % 40) < 20 then
    set_p1('Right', true)
  else
    set_p1('Left', true)
  end
  wait_frames(1)
  polls = polls + 1
  local current_frame = rb(4)
  if current_frame ~= previous_frame then
    advanced = advanced + wrapped_delta(previous_frame, current_frame)
    previous_frame = current_frame
  end
end
clear_p1()

if rb(0) ~= 0x53 or rb(1) ~= 0x4E then
  write_result(false, 'signature changed during long run')
  manager.machine:exit()
  return
end
if rb(2) ~= 2 then
  write_result(false, string.format('state changed during long run: %d', rb(2)))
  manager.machine:exit()
  return
end

local end_frame = rb(4)
if advanced < 240 then
  write_result(false, string.format('game loop stalled: start=%d end=%d advanced=%d polls=%d', start_frame, end_frame, advanced, polls))
  manager.machine:exit()
  return
end

write_result(true, string.format('long run stable: start=%d end=%d advanced=%d polls=%d', start_frame, end_frame, advanced, polls))
manager.machine:exit()
