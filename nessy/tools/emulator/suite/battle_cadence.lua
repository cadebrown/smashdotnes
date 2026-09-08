local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/battle_cadence.txt'

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

local function rb(offset)
  return prg:read_u8(BASE + offset)
end

local function wrapped_delta(before, after)
  local delta = after - before
  if delta < 0 then
    delta = delta + 256
  end
  return delta
end

local function wait_screen_frame()
  local before = screen.frame_number
  repeat
    emu.wait_next_frame()
  until screen.frame_number ~= before
end

local function wait_game_frames(count)
  local previous = rb(4)
  local advanced = 0
  while advanced < count do
    emu.wait_next_frame()
    local current = rb(4)
    advanced = advanced + wrapped_delta(previous, current)
    previous = current
  end
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

local function tap_p1(button)
  clear_inputs()
  p1['P1 ' .. button]:set_value(1)
  wait_game_frames(3)
  clear_inputs()
  wait_game_frames(3)
end

local function drive_active_battle(frame)
  clear_inputs()
  p1['P1 Right']:set_value(frame <= 24 and 1 or 0)
  p2['P2 Left']:set_value(frame <= 24 and 1 or 0)
  p1['P1 A']:set_value((frame % 44) == 26 and 1 or 0)
  p2['P2 A']:set_value((frame % 47) == 28 and 1 or 0)
  p1['P1 B']:set_value((frame % 79) == 45 and 1 or 0)
  p2['P2 B']:set_value((frame % 83) == 49 and 1 or 0)
  p1['P1 Up']:set_value((frame % 131) == 95 and 1 or 0)
  p2['P2 Up']:set_value((frame % 137) == 99 and 1 or 0)
end

local function measure(screen_frames, driver)
  local previous_frame = rb(4)
  local previous_step = rb(21) | (rb(22) << 8)
  local game_updates = 0
  local music_steps = 0
  local active_frames = 0
  local max_projectiles = 0
  local max_oam = 0
  local saw_hit_effect = false
  local stalls = {}
  local catches = {}

  for frame = 1, screen_frames do
    if driver ~= nil then
      driver(frame)
    end
    wait_screen_frame()
    local current_frame = rb(4)
    local current_step = rb(21) | (rb(22) << 8)
    local projectiles = rb(7)
    local presentation = rb(31)
    local update_delta = wrapped_delta(previous_frame, current_frame)
    game_updates = game_updates + update_delta
    if update_delta == 0 then
      table.insert(stalls, string.format(
        '%d(a=%d,s=%d,p=%d,o=%d,d=%d/%d,h=%d/%d,e=%d)',
        frame,
        rb(24) + rb(27),
        rb(25) + rb(28),
        projectiles,
        presentation & 0x3f,
        rb(12),
        rb(18),
        rb(26),
        rb(29),
        presentation >> 6
      ))
    elseif update_delta > 1 then
      table.insert(catches, string.format('%d:+%d', frame, update_delta))
    end
    if current_step ~= previous_step then
      music_steps = music_steps + 1
    end
    if rb(24) > 0 or rb(25) > 0 or rb(27) > 0 or rb(28) > 0 or projectiles > 0 then
      active_frames = active_frames + 1
    end
    if projectiles > max_projectiles then
      max_projectiles = projectiles
    end
    if (presentation & 0x3f) > max_oam then
      max_oam = presentation & 0x3f
    end
    if (presentation & 0xc0) ~= 0 then
      saw_hit_effect = true
    end
    previous_frame = current_frame
    previous_step = current_step
  end

  clear_inputs()
  return game_updates, music_steps, {
    active_frames = active_frames,
    max_projectiles = max_projectiles,
    max_oam = max_oam,
    saw_hit_effect = saw_hit_effect,
    stalls = stalls,
    catches = catches,
  }
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

while rb(0) ~= 0x53 or rb(1) ~= 0x4E do
  wait_screen_frame()
end

local title_updates, title_steps = measure(180)
tap_p1('Start')
tap_p1('Right')
tap_p1('Start')
if rb(2) ~= 2 then
  finish(false, string.format('expected BATTLE state, got %d', rb(2)))
  return
end

local battle_updates, battle_steps, activity = measure(360, drive_active_battle)
if title_updates < 178 or title_updates > 182 then
  finish(false, string.format('title cadence drifted: %d updates in 180 video frames', title_updates))
  return
end
if battle_updates ~= 360 or #activity.stalls ~= 0 or #activity.catches ~= 0 then
  finish(false, string.format(
    'battle cadence drifted: %d updates in 360 video frames; stalls=%s catches=%s',
    battle_updates,
    table.concat(activity.stalls, ','),
    table.concat(activity.catches, ',')
  ))
  return
end
if battle_steps < 90 or battle_steps > 130 then
  finish(false, string.format('battle music cadence drifted: %d event changes in 360 video frames', battle_steps))
  return
end
if activity.active_frames < 90 or activity.max_projectiles < 1 or not activity.saw_hit_effect then
  finish(false, string.format(
    'battle load was too light: active=%d projectiles=%d oam=%d hit_effect=%s',
    activity.active_frames,
    activity.max_projectiles,
    activity.max_oam,
    tostring(activity.saw_hit_effect)
  ))
  return
end

finish(true, string.format(
  'title=%d/180 updates (%d music events); battle=%d/360 updates (%d music events), active=%d projectiles=%d oam=%d hit_effect=%s stalls=%s catches=%s',
  title_updates,
  title_steps,
  battle_updates,
  battle_steps,
  activity.active_frames,
  activity.max_projectiles,
  activity.max_oam,
  tostring(activity.saw_hit_effect),
  table.concat(activity.stalls, ','),
  table.concat(activity.catches, ',')
))
