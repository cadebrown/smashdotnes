local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/fighter_tas.txt'
local artifact_dir = os.getenv('NESSY_ARTIFACT_DIR') or 'out/reports/emulator/artifacts/fighter_tas'
local fighter_name = string.match(out_file, 'tas_([a-z]+)%.txt$')
local fighter_indexes = {mario = 0, simon = 1, tommy = 2, rizer = 3}
local fighter_index = fighter_indexes[fighter_name]
assert(fighter_index ~= nil, 'could not infer fighter from NESSY_TEST_OUT')

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
local p2 = controller(':ctrl2:joypad:JOYPAD', 'P2 ')
local screenshot_count = 0
local trace = {}

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
  local poll_limit = count * 32 + 120
  while advanced < count and polls < poll_limit do
    wait_frames(1)
    polls = polls + 1
    local current = rb(4)
    if current ~= previous then
      advanced = advanced + 1
      previous = current
    end
  end
  return advanced == count
end

local function clear_pad(pad)
  for _, field in pairs(pad) do
    field:set_value(0)
  end
end

local function clear_inputs()
  clear_pad(p1)
  clear_pad(p2)
end

local function tap(pad, prefix, button, frames)
  clear_pad(pad)
  pad[prefix .. button]:set_value(1)
  wait_game_frames(frames or 1)
  clear_pad(pad)
  wait_game_frames(2)
end

local function start_action(pad, prefix, button, timer_offset)
  clear_pad(pad)
  pad[prefix .. button]:set_value(1)
  for _ = 1, 12 do
    if not wait_game_frames(1) then
      clear_pad(pad)
      return false
    end
    if rb(timer_offset) > 0 then
      clear_pad(pad)
      return true
    end
  end
  clear_pad(pad)
  return false
end

local function snapshot(name)
  screen:snapshot(artifact_dir .. '/' .. name)
  screenshot_count = screenshot_count + 1
end

local function finish(ok, message)
  clear_inputs()
  local report = io.open(out_file, 'w')
  if report then
    report:write((ok and 'PASS' or 'FAIL') .. '\n')
    report:write(message .. '\n')
    report:close()
  end
  manager.machine:exit()
end

local function p1_frame()
  return rb(30) & 0x0f
end

local function capture_visible(name, semantic_frame)
  table.insert(trace, string.format('%s=%d@%d', name, semantic_frame, rb(4)))
  if not wait_game_frames(1) then
    return false
  end
  snapshot(name)
  return true
end

local function capture_sequence(expected, limit, label, positions)
  local captured = {}
  local remaining = 0
  for _ in pairs(expected) do
    remaining = remaining + 1
  end

  for _ = 1, limit do
    local frame = p1_frame()
    local name = expected[frame]
    if name and not captured[frame] then
      if positions then
        positions[frame] = rs16(10)
      end
      if not capture_visible(name, frame) then
        return false, label .. ' stopped advancing while capturing frame ' .. frame
      end
      captured[frame] = true
      remaining = remaining - 1
      if remaining == 0 then
        return true, ''
      end
    elseif not wait_game_frames(1) then
      return false, label .. ' stopped advancing'
    end
  end

  local missing = {}
  for frame in pairs(expected) do
    if not captured[frame] then
      table.insert(missing, frame)
    end
  end
  table.sort(missing)
  return false, string.format(
    '%s missed semantic frames %s; current=%d attack=%d special=%d pad=%d',
    label,
    table.concat(missing, ','),
    p1_frame(),
    rb(24),
    rb(25),
    rb(5)
  )
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

wait_frames(120)
if rb(0) ~= 0x53 or rb(1) ~= 0x4e then
  finish(false, 'runtime signature missing')
  return
end

tap(p1, 'P1 ', 'Start', 2)
for _ = 1, fighter_index do
  tap(p1, 'P1 ', 'Right', 1)
end
tap(p1, 'P1 ', 'Start', 2)
if not wait_until(function() return rb(2) == 2 end, 20) then
  finish(false, fighter_name .. ' failed to enter battle')
  return
end

if p1_frame() ~= 0 then
  finish(false, fighter_name .. ' did not start in idle frame 0')
  return
end
if not capture_visible('00-idle.png', 0) then
  finish(false, fighter_name .. ' idle capture stalled')
  return
end

clear_inputs()
p1['P1 Right']:set_value(1)
local ok, message = capture_sequence({
  [1] = '01-walk-a.png',
  [2] = '02-walk-b.png',
  [3] = '03-walk-c.png',
}, 40, fighter_name .. ' walk')
clear_inputs()
if not ok then
  finish(false, message)
  return
end
wait_game_frames(2)

local jump_origin_y = rs16(10)
p1['P1 Up']:set_value(1)
local jump_started = false
for _ = 1, 12 do
  if not wait_game_frames(1) then
    break
  end
  if p1_frame() >= 4 and p1_frame() <= 6 then
    jump_started = true
    break
  end
end
clear_inputs()
if not jump_started then
  finish(false, fighter_name .. ' jump input did not start an action')
  return
end
local jump_positions = {}
local jump_expected = {
  [4] = '04-jump-rise.png',
  [5] = '05-jump-apex.png',
  [6] = '06-jump-fall.png',
}
if fighter_name == 'rizer' then
  jump_expected[4] = '04-jump-spin-a.png'
  jump_expected[5] = '05-jump-spin-b.png'
  jump_expected[6] = '06-jump-spin-c.png'
  jump_expected[14] = '06d-jump-spin-d.png'
end
ok, message = capture_sequence(jump_expected, 90, fighter_name .. ' jump', jump_positions)
if not ok then
  finish(false, message)
  return
end
local jump_height
if fighter_name == 'rizer' then
  local apex_y = jump_origin_y
  for _, y in pairs(jump_positions) do
    if y < apex_y then
      apex_y = y
    end
  end
  local landed = false
  for _ = 1, 100 do
    local y = rs16(10)
    if y < apex_y then
      apex_y = y
    end
    if y == 143 and p1_frame() == 0 then
      landed = true
      break
    end
    if not wait_game_frames(1) then
      break
    end
  end
  jump_height = jump_origin_y - apex_y
  if not landed or jump_height < 48 or jump_height > 96 then
    finish(false, string.format(
      '%s tumble arc invalid: a=%d b=%d c=%d d=%d apex=%d height=%d',
      fighter_name,
      jump_positions[4],
      jump_positions[5],
      jump_positions[6],
      jump_positions[14],
      apex_y,
      jump_height
    ))
    return
  end
else
  jump_height = jump_origin_y - jump_positions[5]
  if jump_positions[4] <= jump_positions[5] or jump_positions[6] <= jump_positions[5] or
      jump_height < 48 or jump_height > 96 then
    finish(false, string.format(
      '%s jump arc invalid: rise=%d apex=%d fall=%d height=%d',
      fighter_name,
      jump_positions[4],
      jump_positions[5],
      jump_positions[6],
      jump_height
    ))
    return
  end
  if not wait_until(function() return rs16(10) == 143 and p1_frame() == 0 end, 100) then
    finish(false, fighter_name .. ' did not land after jump')
    return
  end
end

if not start_action(p1, 'P1 ', 'A', 24) then
  finish(false, fighter_name .. ' jab input did not start an action')
  return
end
ok, message = capture_sequence({
  [7] = '07-jab-start.png',
  [8] = '08-jab-active.png',
  [9] = '09-jab-recover.png',
}, 40, fighter_name .. ' jab')
if not ok then
  finish(false, message)
  return
end
if not wait_until(function() return rb(24) == 0 end, 30) then
  finish(false, fighter_name .. ' jab did not recover')
  return
end

if not start_action(p1, 'P1 ', 'B', 25) then
  finish(false, fighter_name .. ' special input did not start an action')
  return
end
ok, message = capture_sequence({
  [10] = '10-special-start.png',
  [11] = '11-special-active.png',
  [12] = '12-special-recover.png',
}, 50, fighter_name .. ' special')
if not ok then
  finish(false, message)
  return
end
if not wait_until(function() return rb(25) == 0 and rb(7) == 0 and rb(29) == 0 end, 140) then
  finish(false, fighter_name .. ' special did not settle')
  return
end

clear_inputs()
for _ = 1, 100 do
  if rs16(14) - rs16(8) <= 30 then
    break
  end
  p1['P1 Right']:set_value(1)
  p2['P2 Left']:set_value(1)
  wait_game_frames(1)
end
clear_inputs()
wait_game_frames(2)
if rs16(14) - rs16(8) > 36 then
  finish(false, string.format('%s could not stage contact: p1=%d p2=%d', fighter_name, rs16(8), rs16(14)))
  return
end

local damage_before = rb(12)
if not start_action(p2, 'P2 ', 'A', 27) then
  finish(false, fighter_name .. ' opponent jab input did not start an action')
  return
end
if not wait_until(function()
  return rb(12) > damage_before and rb(26) > 0 and p1_frame() == 13 and (rb(31) >> 6) > 0
end, 40) then
  finish(false, string.format(
    '%s hurt setup failed: damage=%d->%d hitstun=%d frame=%d effects=%d',
    fighter_name,
    damage_before,
    rb(12),
    rb(26),
    p1_frame(),
    rb(31) >> 6
  ))
  return
end

if not capture_visible('14-impact.png', 13) then
  finish(false, fighter_name .. ' impact capture stalled')
  return
end
if not wait_until(function()
  return (rb(31) >> 6) == 0 and rb(26) > 0 and p1_frame() == 13
end, 12) then
  finish(false, fighter_name .. ' never exposed a clean hurt pose')
  return
end
if not capture_visible('13-hurt.png', 13) then
  finish(false, fighter_name .. ' hurt capture stalled')
  return
end

if not wait_until(function()
  return rb(26) == 0 and rs16(10) == 143 and p1_frame() == 0
end, 120) then
  finish(false, fighter_name .. ' did not recover to idle')
  return
end
if not capture_visible('15-recovery.png', 0) then
  finish(false, fighter_name .. ' recovery capture stalled')
  return
end

local expected_screenshots = fighter_name == 'rizer' and 17 or 16
if screenshot_count ~= expected_screenshots then
  finish(false, string.format(
    '%s captured %d screenshots, expected %d',
    fighter_name,
    screenshot_count,
    expected_screenshots
  ))
  return
end

local jump_detail
if fighter_name == 'rizer' then
  jump_detail = string.format(
    'tumble=%d/%d/%d/%d (%dpx)',
    jump_positions[4],
    jump_positions[5],
    jump_positions[6],
    jump_positions[14],
    jump_height
  )
else
  jump_detail = string.format(
    'jump=%d/%d/%d (%dpx)',
    jump_positions[4],
    jump_positions[5],
    jump_positions[6],
    jump_height
  )
end

finish(true, string.format(
  '%s TAS captured %d states; %s; damage=%d->%d; trace=%s',
  fighter_name,
  expected_screenshots,
  jump_detail,
  damage_before,
  rb(12),
  table.concat(trace, ',')
))
