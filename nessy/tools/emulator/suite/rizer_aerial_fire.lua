local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/rizer_aerial_fire.txt'
local artifact_dir = os.getenv('NESSY_ARTIFACT_DIR') or 'out/reports/emulator/artifacts/rizer_aerial_fire'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local screen = manager.machine.screens[':screen']
local BASE = 0x06E0

local function controller(tag, prefix)
  local port = manager.machine.ioport.ports[tag]
  local result = {}
  for _, field in pairs(port.fields) do
    local name = tostring(field.name)
    if string.sub(name, 1, #prefix) == prefix and result[name] == nil then
      result[name] = field
    end
  end
  return result
end

local p1 = controller(':ctrl1:joypad:JOYPAD', 'P1 ')

local function rb(offset)
  return prg:read_u8(BASE + offset)
end

local function rs16(offset)
  local value = rb(offset) + rb(offset + 1) * 256
  return value >= 32768 and value - 65536 or value
end

local function wait_frames(count)
  for _ = 1, count do
    emu.wait_next_frame()
  end
end

local function wait_game_frames(count)
  local previous = rb(4)
  local advanced = 0
  local polls = 0
  while advanced < count and polls < count * 32 + 120 do
    wait_frames(1)
    polls = polls + 1
    local current = rb(4)
    if current ~= previous then
      previous = current
      advanced = advanced + 1
    end
  end
  return advanced == count
end

local function clear_p1()
  for _, field in pairs(p1) do
    field:set_value(0)
  end
end

local function tap(button, frames)
  clear_p1()
  p1['P1 ' .. button]:set_value(1)
  wait_game_frames(frames or 1)
  clear_p1()
  wait_game_frames(2)
end

local function wait_until(predicate, limit)
  for _ = 1, limit do
    if predicate() then
      return true
    end
    if not wait_game_frames(1) then
      return false
    end
  end
  return predicate()
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
if rb(0) ~= 0x53 or rb(1) ~= 0x4e then
  finish(false, 'runtime signature missing')
  return
end

tap('Start', 2)
for _ = 1, 3 do
  tap('Right', 1)
end
tap('Start', 2)
if not wait_until(function() return rb(2) == 2 end, 20) then
  finish(false, 'Rizer failed to enter battle')
  return
end

local jump_origin = rs16(10)
clear_p1()
p1['P1 Up']:set_value(1)
if not wait_game_frames(1) then
  finish(false, 'jump input stalled')
  return
end
clear_p1()
if not wait_until(function()
  local frame = rb(30) & 0x0f
  return rs16(10) < jump_origin and (frame == 4 or frame == 5 or frame == 6 or frame == 14)
end, 12) then
  finish(false, 'Rizer did not enter the authored tumble loop')
  return
end

p1['P1 B']:set_value(1)
local fired = false
for _ = 1, 12 do
  if not wait_game_frames(1) then
    break
  end
  if rb(25) > 0 and rb(7) > 0 then
    fired = true
    break
  end
end
clear_p1()
if not fired then
  finish(false, string.format('aerial fire did not start: special=%d projectiles=%d', rb(25), rb(7)))
  return
end

local names = {
  [4] = '00-spin-a-with-bullet.png',
  [5] = '01-spin-b-with-bullet.png',
  [6] = '02-spin-c-with-bullet.png',
  [14] = '03-spin-d-with-bullet.png',
}
local seen = {}
local special_seen = {}
local trace = {}
local min_y = rs16(10)
local projectile_seen = rb(7) > 0

for _ = 1, 64 do
  local y = rs16(10)
  local frame = rb(30) & 0x0f
  if y < min_y then
    min_y = y
  end
  if y < jump_origin then
    if names[frame] == nil then
      finish(false, string.format(
        'airborne render left the tumble loop: frame=%d special=%d projectile=%d',
        frame,
        rb(25),
        rb(7)
      ))
      return
    end
    seen[frame] = true
    if rb(25) > 0 then
      special_seen[frame] = true
    end
    if rb(7) > 0 then
      projectile_seen = true
      if not seen['shot-' .. frame] then
        if not wait_game_frames(1) then
          finish(false, 'capture stalled')
          return
        end
        screen:snapshot(artifact_dir .. '/' .. names[frame])
        seen['shot-' .. frame] = true
      end
    end
    table.insert(trace, string.format('%d:%d:%d', frame, rb(25), rb(7)))
  elseif y == jump_origin then
    break
  end
  if not wait_game_frames(1) then
    finish(false, 'airborne observation stalled')
    return
  end
end

local spin_count = 0
local special_spin_count = 0
for frame in pairs(names) do
  if seen[frame] then
    spin_count = spin_count + 1
  end
  if special_seen[frame] then
    special_spin_count = special_spin_count + 1
  end
  if not seen['shot-' .. frame] then
    finish(false, string.format('spin frame %d was never visible with its bullet in flight', frame))
    return
  end
end

if spin_count ~= 4 or special_spin_count < 2 or not projectile_seen then
  finish(false, string.format(
    'incomplete aerial fire: spin=%d special_spin=%d projectile=%s',
    spin_count,
    special_spin_count,
    tostring(projectile_seen)
  ))
  return
end
if jump_origin - min_y < 48 or jump_origin - min_y > 96 then
  finish(false, string.format('jump height invalid: origin=%d apex=%d', jump_origin, min_y))
  return
end
if not wait_until(function() return rs16(10) == jump_origin and (rb(30) & 0x0f) == 0 end, 80) then
  finish(false, 'Rizer did not recover to idle after aerial fire')
  return
end

finish(true, string.format(
  'Rizer kept all four tumble poses through aerial fire; special poses=%d; height=%d; trace=%s',
  special_spin_count,
  jump_origin - min_y,
  table.concat(trace, ',')
))
