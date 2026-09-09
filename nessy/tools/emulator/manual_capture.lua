-- Input-only manual illustration drive.  It consumes the published $06E0
-- diagnostic ABI; it never writes ROM/RAM state.
local capture_dir = assert(os.getenv('NESSY_CAPTURE_DIR'), 'NESSY_CAPTURE_DIR is required')
local report_file = assert(os.getenv('NESSY_CAPTURE_REPORT'), 'NESSY_CAPTURE_REPORT is required')
local state_file = capture_dir .. '/capture-state.json'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local screen = manager.machine.screens[':screen']
local BASE = 0x06E0
local BTN_UP = 0x10
local captures = {}
local double_jump_observation = nil
-- Character selection persists across returns to title; retain the selected
-- slot so later captures select an absolute roster member, not a delta.
local selected_fighter = 0

local function controller(tag, prefix)
  local result = {}
  for _, field in pairs(manager.machine.ioport.ports[tag].fields) do
    local name = tostring(field.name)
    if name:sub(1, #prefix) == prefix and result[name:sub(#prefix + 1)] == nil then
      result[name:sub(#prefix + 1)] = field
    end
  end
  return result
end

local p1 = controller(':ctrl1:joypad:JOYPAD', 'P1 ')
local p2 = controller(':ctrl2:joypad:JOYPAD', 'P2 ')

local function rb(offset) return prg:read_u8(BASE + offset) end
local function rs16(offset)
  local value = rb(offset) + rb(offset + 1) * 256
  return value >= 32768 and value - 65536 or value
end

local function set_pad(pad, buttons)
  for _, field in pairs(pad) do field:set_value(0) end
  for _, button in ipairs(buttons or {}) do
    assert(pad[button], 'unknown controller button ' .. button):set_value(1)
  end
end
local function clear_inputs() set_pad(p1); set_pad(p2) end

-- Wait for simulation ticks, rather than trusting arbitrary video-frame delays.
local function tick(count)
  for _ = 1, count or 1 do
    local before = rb(4)
    local advanced = false
    for _ = 1, 152 do
      emu.wait_next_frame()
      if rb(4) ~= before then advanced = true; break end
    end
    assert(advanced, 'game frame counter stopped advancing')
  end
end

local function wait_until(predicate, limit, label)
  for _ = 1, limit do
    if predicate() then return end
    tick(1)
  end
  assert(predicate(), 'timed out waiting for ' .. label)
end

local function tap(pad, button, frames)
  set_pad(pad, {button})
  tick(frames or 1)
  set_pad(pad)
  tick(2)
end

local function json_string(value)
  return '"' .. value:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
end

local function capture(name, explanation, fighter)
  -- The ABI state describes OAM prepared during this simulation tick.  Advance
  -- once so that exact OAM reaches DMA/PPU, then snapshot; the JSON deliberately
  -- retains the state observed *before* that extra display tick.
  table.insert(captures, {
    name = name,
    frame = rb(4),
    scene = rb(2), mode = rb(3),
    p1x = rs16(8), p1y = rs16(10), p1damage = rb(12), p1ko = rb(13),
    p2x = rs16(14), p2y = rs16(16), p2damage = rb(18), p2ko = rb(19),
    p1input = rb(5), p2input = rb(6),
    p1frame = rb(30) & 0x0f, p2frame = rb(30) >> 4,
    p1jab = rb(24), p1special = rb(25), p1hitstun = rb(26),
    p2jab = rb(27), p2special = rb(28), p2hitstun = rb(29),
    hit_effects = rb(31) >> 6,
    explanation = explanation,
    fighter = fighter or 'mario',
  })
  tick(1)
  screen:snapshot(capture_dir .. '/' .. name)
end

local function write_state()
  local file = assert(io.open(state_file, 'w'))
  file:write('{\n  "abi_base":"0x06E0",\n  "input_only":true,\n')
  file:write('  "screenshot_delay":"Each PNG is taken one game tick after its recorded predicate state, after prepared OAM reaches DMA/PPU.",\n')
  if double_jump_observation then
    file:write(string.format(
      '  "double_jump_observation":{"release_y":%d,"release_input":%d,"second_press_y":%d,"second_press_input":%d},\n',
      double_jump_observation.release_y, double_jump_observation.release_input,
      double_jump_observation.second_press_y, double_jump_observation.second_press_input))
  end
  file:write('  "captures":[\n')
  for index, item in ipairs(captures) do
    file:write(string.format(
      '    {"file":%s,"fighter":%s,"scene":%d,"mode":%d,"game_frame":%d,"p1":{"x":%d,"y":%d,"damage":%d,"ko":%d,"input":%d,"animation":%d,"jab":%d,"special":%d,"hitstun":%d},"p2":{"x":%d,"y":%d,"damage":%d,"ko":%d,"input":%d,"animation":%d,"jab":%d,"special":%d,"hitstun":%d},"hit_effects":%d,"observed":%s}%s\n',
      json_string(item.name), json_string(item.fighter), item.scene, item.mode, item.frame,
      item.p1x, item.p1y, item.p1damage, item.p1ko, item.p1input, item.p1frame, item.p1jab, item.p1special, item.p1hitstun,
      item.p2x, item.p2y, item.p2damage, item.p2ko, item.p2input, item.p2frame, item.p2jab, item.p2special, item.p2hitstun,
      item.hit_effects, json_string(item.explanation),
      index == #captures and '' or ','))
  end
  file:write('  ]\n}\n')
  file:close()
end

local function finish(ok, message)
  clear_inputs()
  write_state()
  local report = assert(io.open(report_file, 'w'))
  report:write((ok and 'PASS' or 'FAIL') .. '\n' .. message .. '\n')
  report:close()
  manager.machine:exit()
end

local function return_to_title()
  set_pad(p1, {'Start', 'Select'})
  wait_until(function() return rb(2) == 0 end, 20, 'return to title')
  clear_inputs()
  tick(2)
end

local function start_fighter(fighter_index)
  return_to_title()
  tap(p1, 'Start', 1)
  wait_until(function() return rb(2) == 1 end, 20, 'fighter lobby')
  local steps = (fighter_index - selected_fighter) % 4
  for _ = 1, steps do tap(p1, 'Right', 1) end
  selected_fighter = fighter_index
  tap(p1, 'Start', 1)
  wait_until(function() return rb(2) == 2 end, 20, 'battle for selected fighter')
  assert((rb(30) & 0x0f) == 0, 'selected fighter did not start at semantic idle frame')
end

local function capture_special(name, fighter, projectile, airborne)
  if airborne then
    set_pad(p1, {'Up'})
    tick(1)
    wait_until(function() return rs16(10) < 143 and (rb(5) & BTN_UP) ~= 0 end, 12, fighter .. ' jump')
    set_pad(p1)
  end
  set_pad(p1, {'B'})
  wait_until(function() return rb(25) > 0 end, 12, fighter .. ' special startup')
  set_pad(p1)
  if airborne then
    wait_until(function()
      local frame = rb(30) & 0x0f
      return rs16(10) < 143 and (frame == 4 or frame == 5 or frame == 6 or frame == 14) and rb(7) > 0
    end, 50, fighter .. ' airborne curl with live bullet')
  else
    wait_until(function()
      return (rb(30) & 0x0f) == 11 and (not projectile or rb(7) > 0)
    end, 50, fighter .. ' semantic special-active frame')
  end
  capture(name,
    airborne and (fighter .. ' was selected through the lobby; airborne curl frame, active special timer, and live bullet were observed.') or
      (fighter .. ' was selected through the lobby; semantic P1 frame 11 and active special timer were observed' ..
        (projectile and ' with an active projectile.' or '.')), fighter)
  wait_until(function() return rb(25) == 0 and rb(7) == 0 and (not airborne or rs16(10) == 143) end, 150, fighter .. ' special recovery')
end

local ok, message = pcall(function()
  for _ = 1, 120 do emu.wait_next_frame() end
  assert(rb(0) == 0x53 and rb(1) == 0x4e and rb(2) == 0, 'expected healthy title state')
  capture('01_title.png', 'ABI signature SN and title scene (state=0) observed.')

  tap(p1, 'Start', 2)
  wait_until(function() return rb(2) == 1 end, 30, 'lobby scene')
  capture('02_lobby.png', 'P1 Start advanced the observed scene to lobby (state=1).')

  tap(p1, 'Start', 2)
  wait_until(function() return rb(2) == 2 end, 30, 'battle scene')
  capture('03_battle.png', 'P1 Start advanced the observed scene to battle (state=2).')

  set_pad(p1, {'Right'})
  tick(8)
  assert(rs16(8) > 56, 'Right input did not move P1')
  capture('04_walk.png', 'P1 x increased from its 56px spawn under held Right input.')
  clear_inputs()
  wait_until(function() return rs16(10) == 143 end, 60, 'grounded neutral state')

  tap(p1, 'Up', 1)
  wait_until(function() return rs16(10) < 143 end, 12, 'first jump ascent')
  capture('05_jump.png', 'A fresh Up press produced airborne P1 y < 143.')
  wait_until(function() return rs16(10) == 143 end, 120, 'landing after base jump')

  tap(p1, 'A', 1)
  wait_until(function() return rb(24) > 0 end, 12, 'jab startup')
  local jab_damage_before = rb(18)
  wait_until(function()
    local frame = rb(30) & 0x0f
    return frame == 8 and rb(24) >= 4 and rb(24) <= 6 and rb(18) == jab_damage_before and (rb(31) >> 6) == 0
  end, 20, 'Mario extended active jab that misses')
  capture('06_jab.png', 'Mario reached semantic active jab frame 8 (timer 6..4) while P2 damage and hit effects remained zero: the extended kick misses.')
  wait_until(function() return rb(24) == 0 end, 40, 'jab recovery')

  tap(p1, 'B', 1)
  wait_until(function() return rb(25) > 0 and rb(7) > 0 end, 12, 'Mario special projectile')
  capture('07_special.png', 'P1 special timer and active projectile count were both nonzero.')
  wait_until(function() return rb(25) == 0 and rb(7) == 0 and rs16(10) == 143 end, 150, 'special recovery')
  -- Mario's fireball may contact the stationary opponent after its screenshot.
  -- Start a fresh neutral Mario match so the jump triptych is uncontaminated.
  start_fighter(0)

  -- Three frames make the release requirement visible: first press, no Up,
  -- then a second fresh Up edge that restarts upward motion.
  set_pad(p1, {'Up'})
  tick(1)
  wait_until(function() return rs16(10) < 143 and (rb(5) & BTN_UP) ~= 0 end, 10, 'first jump press')
  capture('08_jump_press.png', 'First Up press sampled by ABI and P1 became airborne.')
  set_pad(p1)
  wait_until(function() return (rb(5) & BTN_UP) == 0 end, 10, 'Up release sampled by game')
  local previous_y = rs16(10)
  local released_y = previous_y
  local saw_first_descent = false
  for _ = 1, 40 do
    tick(1)
    assert((rb(5) & BTN_UP) == 0, 'Up was re-sampled during the released-airborne wait')
    local current_y = rs16(10)
    if current_y >= previous_y then
      released_y = current_y
      saw_first_descent = true
      break
    end
    previous_y = current_y
  end
  assert(saw_first_descent, 'first jump did not reach an apex/descending air state')
  wait_until(function()
    return (rb(5) & BTN_UP) == 0 and rs16(10) >= 108 and rs16(10) < 143
  end, 40, 'clearly descending released-airborne frame')
  released_y = rs16(10)
  capture('09_jump_release.png', 'Up remained released through the apex until P1 was visibly falling (y >= 108) while still airborne.')
  set_pad(p1, {'Up'})
  wait_until(function() return (rb(5) & BTN_UP) ~= 0 and rs16(10) < released_y end, 10, 'fresh second jump boost')
  local second_press_y = rs16(10)
  double_jump_observation = {
    release_y = released_y, release_input = 0,
    second_press_y = second_press_y, second_press_input = rb(5),
  }
  tick(3)
  assert((rb(5) & BTN_UP) ~= 0 and rs16(10) < second_press_y, 'held second jump did not continue upward before screenshot')
  capture('10_jump_again.png', 'After the released falling frame, a fresh sampled Up press renewed upward motion; this frame holds Up three ticks into the visible boost.')
  clear_inputs()
  wait_until(function() return rs16(10) == 143 end, 120, 'landing after double jump')

  -- Stage a real Mario-versus-Simon jab with neutral P2 input.  This avoids a
  -- simultaneous trade: the illustration must show a single observed hit.
  set_pad(p1, {'Right'})
  set_pad(p2, {'Left'})
  wait_until(function() return rs16(14) - rs16(8) <= 24 end, 80, 'melee contact distance')
  clear_inputs()
  tick(2)
  local damage_before = rb(18)
  set_pad(p1, {'A'})
  wait_until(function() return rb(24) > 0 end, 12, 'P1 jab startup')
  set_pad(p1)
  wait_until(function()
    return rb(12) == 0 and rb(18) > damage_before and rb(29) > 0 and (rb(31) >> 6) > 0
  end, 20, 'successful hit with damage, hitstun, and spark')
  capture('11_hit.png', 'Neutral P2 took a Mario jab: P2 damage increased, P2 hitstun was nonzero, P1 damage stayed zero, and an active hit effect was exported.')

  start_fighter(1)
  capture_special('14_simon_special.png', 'simon', true)
  start_fighter(2)
  capture_special('15_tommy_special.png', 'tommy', false)
  start_fighter(3)
  capture_special('16_rizer_special.png', 'rizer', true, true)

  return 'captured base scenes, a release-gated double jump, an input-only successful jab, and Simon/Tommy/Rizer special-active frames'
end)

finish(ok, tostring(message))
