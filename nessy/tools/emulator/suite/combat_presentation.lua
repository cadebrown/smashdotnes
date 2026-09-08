local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/combat_presentation.txt'
local artifact_dir = os.getenv('NESSY_ARTIFACT_DIR') or 'out/reports/emulator/artifacts/combat_presentation'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local screen = manager.machine.screens[':screen']
local BASE = 0x06E0

local p1_port = manager.machine.ioport.ports[':ctrl1:joypad:JOYPAD']
local p1 = {}
for _, field in pairs(p1_port.fields) do
  local name = tostring(field.name)
  if string.sub(name, 1, 3) == 'P1 ' and p1[name] == nil then
    p1[name] = field
  end
end

local function rb(off)
  return prg:read_u8(BASE + off)
end

local function rs16(off)
  local value = rb(off) + rb(off + 1) * 256
  return value >= 32768 and value - 65536 or value
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

local function clear_p1()
  for _, field in pairs(p1) do
    field:set_value(0)
  end
end

local function tap_p1(button, frames)
  clear_p1()
  p1['P1 ' .. button]:set_value(1)
  wait_game_frames(frames or 2)
  clear_p1()
  wait_game_frames(2)
end

local function finish(ok, message)
  clear_p1()
  local report = io.open(out_file, 'w')
  if report then
    report:write((ok and 'PASS' or 'FAIL') .. '\n')
    report:write(message .. '\n')
    report:close()
  end
  manager.machine:exit()
end

wait_frames(120)
tap_p1('Start', 3)
wait_game_frames(3)
tap_p1('Start', 3)
wait_game_frames(3)
if rb(2) ~= 2 then
  finish(false, 'failed to reach battle')
  return
end

screen:snapshot(artifact_dir .. '/01_neutral.png')
if (rb(31) >> 6) ~= 0 then
  finish(false, 'hit effect exists before contact')
  return
end

clear_p1()
p1['P1 Right']:set_value(1)
for _ = 1, 80 do
  if rs16(14) - rs16(8) <= 24 then
    break
  end
  wait_game_frames(1)
end
clear_p1()
wait_game_frames(2)
screen:snapshot(artifact_dir .. '/02_approach.png')

local damage_before = rb(18)
clear_p1()
p1['P1 A']:set_value(1)
for _ = 1, 12 do
  wait_game_frames(1)
  if rb(24) > 0 then
    break
  end
end
clear_p1()
for _ = 1, 16 do
  if rb(18) > damage_before then
    break
  end
  wait_game_frames(1)
end
screen:snapshot(artifact_dir .. '/03_impact.png')
local damage_after = rb(18)

if damage_after <= damage_before then
  finish(false, string.format(
    'jab did not deal damage: before=%d after=%d p1x=%d p2x=%d attack=%d pad=%d',
    damage_before,
    damage_after,
    rs16(8),
    rs16(14),
    rb(24),
    rb(5)
  ))
  return
end
if (rb(31) >> 6) == 0 or rb(23) ~= 3 then
  finish(false, string.format('impact presentation missing: effects=%d cue=%d', rb(31) >> 6, rb(23)))
  return
end
if rb(28) ~= 0 or rb(27) ~= 0 then
  finish(false, 'hit target retained an action timer')
  return
end

wait_game_frames(20)
screen:snapshot(artifact_dir .. '/04_recovery.png')
if (rb(31) >> 6) ~= 0 then
  finish(false, 'impact effect did not expire')
  return
end

finish(true, string.format(
  'jab impact synchronized: damage=%d->%d cue=%d max_oam=%d',
  damage_before,
  damage_after,
  rb(23),
  rb(31) & 0x3f
))
