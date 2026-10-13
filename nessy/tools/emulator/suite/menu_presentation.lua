local out_file = assert(os.getenv('NESSY_TEST_OUT'))
local artifact_dir = assert(os.getenv('NESSY_ARTIFACT_DIR'))
local prg = manager.machine.devices[':maincpu'].spaces['program']
local screen = manager.machine.screens[':screen']
local BASE = 0x06E0
local OAM = 0x0200

local p1 = {}
for _, field in pairs(manager.machine.ioport.ports[':ctrl1:joypad:JOYPAD'].fields) do
  local name = tostring(field.name)
  if name:sub(1, 3) == 'P1 ' and p1[name] == nil then p1[name] = field end
end

local function rb(offset) return prg:read_u8(BASE + offset) end
local function oam_rb(offset) return prg:read_u8(OAM + offset) end

local function clear()
  for _, field in pairs(p1) do field:set_value(0) end
end

local function tick(count)
  for _ = 1, count do emu.wait_next_frame() end
end

local function tick_game_frames(count)
  local previous = rb(4)
  local advanced = 0
  local polls = 0
  while advanced < count and polls < count * 32 + 120 do
    tick(1)
    polls = polls + 1
    if rb(4) ~= previous then
      advanced = advanced + 1
      previous = rb(4)
    end
  end
  return advanced == count
end

local function tap(button)
  clear()
  p1['P1 ' .. button]:set_value(1)
  assert(tick_game_frames(3), 'game frame counter stopped while holding ' .. button)
  clear()
  assert(tick_game_frames(3), 'game frame counter stopped after ' .. button)
end

local function fail(message)
  clear()
  local report = assert(io.open(out_file, 'w'))
  report:write('FAIL\n' .. message .. '\n')
  report:close()
  manager.machine:exit()
end

local function require_scene(scene, name)
  if rb(2) ~= scene then
    fail(string.format('expected %s scene %d, got %d', name, scene, rb(2)))
    return false
  end
  return true
end

-- The runtime publishes live OAM record count in $06ff. Inspect the actual
-- $0200 buffer too, so cadence proves changed sprites rather than a counter.
local function inspect_oam(name, limit)
  local sprites = rb(31) & 0x3f
  local max_scanline = 0
  local signature = {}

  if sprites == 0 or sprites > limit then
    fail(string.format('%s OAM budget invalid: %d sprites (expected 1..%d)', name, sprites, limit))
    return nil
  end

  for i = 0, sprites - 1 do
    local offset = i * 4
    signature[#signature + 1] = string.format('%02x%02x%02x%02x',
      oam_rb(offset), oam_rb(offset + 1), oam_rb(offset + 2), oam_rb(offset + 3))
  end
  for scanline = 0, 239 do
    local occupied = 0
    for i = 0, sprites - 1 do
      local y = oam_rb(i * 4)
      -- OAM Y is the scanline immediately above an 8-pixel sprite.
      if scanline > y and scanline <= y + 8 then occupied = occupied + 1 end
    end
    if occupied > max_scanline then max_scanline = occupied end
  end
  if max_scanline > 8 then
    fail(string.format('%s exceeds the NES 8-sprite scanline limit: %d', name, max_scanline))
    return nil
  end
  return {signature = table.concat(signature, ':'), sprites = sprites, max_scanline = max_scanline}
end

local function sample_oam(name, scene, limit, frames)
  local first = inspect_oam(name, limit)
  if first == nil then return nil end
  local worst_sprites = first.sprites
  local worst_scanline = first.max_scanline

  for _ = 1, frames do
    if not tick_game_frames(1) then
      fail(name .. ' stopped advancing during OAM sampling')
      return nil
    end
    if not require_scene(scene, name) then return nil end
    local current = inspect_oam(name, limit)
    if current == nil then return nil end
    if current.sprites > worst_sprites then worst_sprites = current.sprites end
    if current.max_scanline > worst_scanline then worst_scanline = current.max_scanline end
  end

  return {max_sprites = worst_sprites, max_scanline = worst_scanline}
end

for _ = 1, 180 do
  tick(1)
  if rb(0) == 0x53 and rb(1) == 0x4e then break end
end
if rb(0) ~= 0x53 or rb(1) ~= 0x4e then
  fail('runtime signature missing')
  return
end
if not require_scene(0, 'TITLE') then return end
local title_early = inspect_oam('title entrance', 24)
if title_early == nil then return end
screen:snapshot(artifact_dir .. '/title-entrance-a.png')
if not tick_game_frames(16) then
  fail('title stopped advancing during early entrance')
  return
end
if not require_scene(0, 'TITLE') then return end
local title_late = inspect_oam('title entrance', 24)
if title_late == nil then return end
if title_late.signature == title_early.signature then
  fail('title entrance did not alter OAM bytes across 16 frames')
  return
end
screen:snapshot(artifact_dir .. '/title-entrance-b.png')

-- Start is sent while Rizer is still entering: animation never gates input.
tap('Start')
if not require_scene(1, 'LOBBY') then return end
local lobby_early = inspect_oam('lobby', 48)
if lobby_early == nil then return end
screen:snapshot(artifact_dir .. '/lobby-a.png')
local lobby_first_profile = sample_oam('lobby', 1, 48, 32)
if lobby_first_profile == nil then return end
if not require_scene(1, 'LOBBY') then return end
local lobby_late = inspect_oam('lobby', 48)
if lobby_late == nil then return end
if lobby_late.signature == lobby_early.signature then
  fail('lobby portrait cadence did not alter OAM bytes across 32 frames')
  return
end
screen:snapshot(artifact_dir .. '/lobby-b.png')

-- Visit every staggered portrait phase, checking the actual OAM buffer each
-- game frame rather than treating two anchors as a budget proof.
local lobby_tail_profile = sample_oam('lobby', 1, 48, 96)
if lobby_tail_profile == nil then return end
local lobby_profile = {
  max_sprites = math.max(lobby_first_profile.max_sprites, lobby_tail_profile.max_sprites),
  max_scanline = math.max(lobby_first_profile.max_scanline, lobby_tail_profile.max_scanline),
}

tap('Select')
if not require_scene(0, 'TITLE') then return end
-- A return resets the timer. Sample its full 96-frame entrance and another
-- 128 frames of idle cadence to cover the complete presentation loop.
local title_profile = sample_oam('title return', 0, 24, 224)
if title_profile == nil then return end

clear()
local report = assert(io.open(out_file, 'w'))
report:write(string.format(
  'PASS\ntitle 96-frame entrance + 128-frame idle: OAM<=%d, scanline<=%d; lobby 128 frames: OAM<=%d, scanline<=%d; early Start and Select return preserved\n',
  title_profile.max_sprites, title_profile.max_scanline,
  lobby_profile.max_sprites, lobby_profile.max_scanline
))
report:close()
manager.machine:exit()
