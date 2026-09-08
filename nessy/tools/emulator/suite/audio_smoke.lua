local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/audio_smoke.txt'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local BASE = 0x06E0
local writes = {}

local p1_port = manager.machine.ioport.ports[':ctrl1:joypad:JOYPAD']
local p1 = {}
for _, field in pairs(p1_port.fields) do
  local name = tostring(field.name)
  if string.sub(name, 1, 3) == 'P1 ' and p1[name] == nil then
    p1[name] = field
  end
end

local tap = prg:install_write_tap(0x4000, 0x4017, 'smash_audio_writes', function(offset, _data, _mask)
  writes[offset] = (writes[offset] or 0) + 1
end)

local function rb(off)
  return prg:read_u8(BASE + off)
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
  tap:remove()
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
if rb(20) ~= 0 then
  finish(false, string.format('expected title music scene 0, got %d', rb(20)))
  return
end

local title_step = rb(21)
wait_game_frames(24)
if rb(21) == title_step then
  finish(false, 'title sequencer did not advance')
  return
end

tap_p1('Start', 3)
wait_game_frames(3)
if rb(2) ~= 1 or rb(20) ~= 1 then
  finish(false, 'lobby state and music scene did not change together')
  return
end

tap_p1('Start', 3)
wait_game_frames(3)
if rb(2) ~= 2 or rb(20) ~= 2 then
  finish(false, 'battle state and music scene did not change together')
  return
end

clear_p1()
p1['P1 Up']:set_value(1)
for _ = 1, 12 do
  wait_game_frames(1)
  if rb(23) == 2 then
    break
  end
end
clear_p1()
if rb(23) ~= 2 then
  finish(false, string.format('expected jump cue 2, got %d', rb(23)))
  return
end

if not wait_game_frames(180) then
  finish(false, 'gameplay clock stalled during sustained audio observation')
  return
end

local required = {0x4000, 0x4002, 0x4004, 0x4006, 0x4008, 0x400A, 0x400B, 0x400C, 0x400E, 0x400F}
for _, address in ipairs(required) do
  if (writes[address] or 0) == 0 then
    finish(false, string.format('no writes observed at APU register $%04X', address))
    return
  end
end

if (writes[0x4003] or 0) + (writes[0x4007] or 0) == 0 then
  finish(false, 'no pulse timer-high transition was observed')
  return
end

if (writes[0x4003] or 0) >= (writes[0x4002] or 0) then
  finish(false, string.format(
    'pulse 1 retriggered its phase for every pitch change: low=%d high=%d',
    writes[0x4002] or 0,
    writes[0x4003] or 0
  ))
  return
end
if (writes[0x4007] or 0) >= (writes[0x4006] or 0) then
  finish(false, string.format(
    'pulse 2 retriggered its phase for every pitch change: low=%d high=%d',
    writes[0x4006] or 0,
    writes[0x4007] or 0
  ))
  return
end

finish(true, string.format(
  'three music scenes advanced; pulse1=%d/%d pulse2=%d/%d triangle=%d noise=%d jump_cue=%d',
  writes[0x4002] or 0,
  writes[0x4003] or 0,
  writes[0x4006] or 0,
  writes[0x4007] or 0,
  writes[0x400A] or 0,
  writes[0x400E] or 0,
  rb(23)
))
