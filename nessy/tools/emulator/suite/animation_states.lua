local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/animation_states.txt'
local artifact_dir = os.getenv('NESSY_ARTIFACT_DIR') or 'out/reports/emulator/artifacts/animation_states'

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

local function rb(offset)
  return prg:read_u8(BASE + offset)
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

local function clear_p1()
  for _, field in pairs(p1) do
    field:set_value(0)
  end
end

local function tap(button, frames)
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

local function observe_tick(seen, screenshot_prefix)
  if not wait_game_frames(1) then
    return false
  end
  if screenshot_prefix and seen.pending_frame ~= nil and not seen['shot-' .. seen.pending_frame] then
    screen:snapshot(string.format(
      '%s/%s-frame-%02d.png',
      artifact_dir,
      screenshot_prefix,
      seen.pending_frame
    ))
    seen['shot-' .. seen.pending_frame] = true
  end
  local frame = rb(30) & 0x0f
  seen[frame] = true
  table.insert(seen.trace, string.format('%d:%d', rb(24), frame))
  seen.pending_frame = frame
  return true
end

local function observe_frames(count, screenshot_prefix, seen)
  seen = seen or {trace = {}}
  for _ = 1, count do
    if not observe_tick(seen, screenshot_prefix) then
      break
    end
  end
  return seen
end

local function start_action(button, timer_offset, screenshot_prefix)
  local seen = {trace = {}}
  clear_p1()
  p1['P1 ' .. button]:set_value(1)
  for _ = 1, 12 do
    observe_tick(seen, screenshot_prefix)
    if rb(timer_offset) > 0 then
      clear_p1()
      return seen
    end
  end
  clear_p1()
  return seen
end

local function require_frames(seen, expected, label)
  for _, frame in ipairs(expected) do
    if not seen[frame] then
      local observed = {}
      for value, present in pairs(seen) do
        if present and type(value) == 'number' then
          table.insert(observed, value)
        end
      end
      table.sort(observed)
      return false, string.format(
        '%s never rendered semantic frame %d; observed=%s',
        label,
        frame,
        table.concat(observed, ',') .. ' trace=' .. table.concat(seen.trace, ',')
      )
    end
  end
  return true, ''
end

local names = {'mario', 'simon', 'tommy', 'rizer'}

wait_frames(120)
tap('Start', 3)
wait_game_frames(3)

for character = 0, 3 do
  if character > 0 then
    tap('Right', 2)
  end
  tap('Start', 3)
  wait_game_frames(3)
  if rb(2) ~= 2 then
    finish(false, names[character + 1] .. ' failed to enter battle')
    return
  end

  if (rb(30) & 0x0f) ~= 0 then
    finish(false, names[character + 1] .. ' did not begin in idle frame 0')
    return
  end
  screen:snapshot(artifact_dir .. '/' .. names[character + 1] .. '-idle.png')

  clear_p1()
  p1['P1 Right']:set_value(1)
  local walk = observe_frames(20, names[character + 1] .. '-walk')
  clear_p1()
  wait_game_frames(2)
  local ok, message = require_frames(walk, {1, 2, 3}, names[character + 1] .. ' walk')
  if not ok then
    finish(false, message)
    return
  end

  local jump = {trace = {}}
  clear_p1()
  p1['P1 Up']:set_value(1)
  for _ = 1, 12 do
    observe_tick(jump, names[character + 1] .. '-jump')
    local frame = rb(30) & 0x0f
    if frame >= 4 and frame <= 6 then
      break
    end
  end
  clear_p1()
  observe_frames(60, names[character + 1] .. '-jump', jump)
  local jump_frames = character == 3 and {4, 5, 6, 14} or {4, 5, 6}
  ok, message = require_frames(jump, jump_frames, names[character + 1] .. ' jump')
  if not ok then
    finish(false, message)
    return
  end
  wait_game_frames(12)

  local jab = start_action('A', 24, names[character + 1] .. '-jab')
  observe_frames(30, names[character + 1] .. '-jab', jab)
  ok, message = require_frames(jab, {7, 8, 9}, names[character + 1] .. ' jab')
  if not ok then
    finish(false, message)
    return
  end
  wait_game_frames(2)

  local special = start_action('B', 25, names[character + 1] .. '-special')
  observe_frames(30, names[character + 1] .. '-special', special)
  ok, message = require_frames(special, {10, 11, 12}, names[character + 1] .. ' special')
  if not ok then
    finish(false, message)
    return
  end

  clear_p1()
  p1['P1 Start']:set_value(1)
  p1['P1 Select']:set_value(1)
  wait_game_frames(2)
  clear_p1()
  wait_game_frames(3)
  if rb(2) ~= 0 then
    finish(false, names[character + 1] .. ' failed to return to title')
    return
  end
  tap('Start', 3)
  wait_game_frames(3)
end

finish(true, 'all fighters rendered every authored walk, jump, jab, and special pose')
