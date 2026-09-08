local capture_dir = os.getenv('NESSY_CAPTURE_DIR') or 'out/snaps'
local report_file = os.getenv('NESSY_CAPTURE_REPORT') or 'out/reports/emulator/capture.txt'

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

local function wait_frames(n)
  for _ = 1, n do
    emu.wait_next_frame()
  end
end

local function set_p1(button, down)
  local field = p1['P1 ' .. button]
  if field ~= nil then
    field:set_value(down and 1 or 0)
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
  wait_frames(hold_frames or 2)
  clear_p1()
  wait_frames(2)
end

local function capture(name)
  screen:snapshot(capture_dir .. '/' .. name)
end

local function finish(ok, message)
  clear_p1()
  local report = io.open(report_file, 'w')
  if report then
    report:write((ok and 'PASS' or 'FAIL') .. '\n')
    report:write(message .. '\n')
    report:close()
  end
  manager.machine:exit()
end

wait_frames(240)
if rb(0) ~= 0x53 or rb(1) ~= 0x4E or rb(2) ~= 0 then
  finish(false, 'expected a healthy TITLE state before capture')
  return
end
capture('01_title.png')

tap_p1('Start', 3)
wait_frames(180)
if rb(2) ~= 1 then
  finish(false, 'expected LOBBY state after first START')
  return
end
capture('02_lobby.png')

tap_p1('Start', 3)
wait_frames(180)
if rb(2) ~= 2 then
  finish(false, 'expected BATTLE state after second START')
  return
end
capture('03_battle.png')

clear_p1()
set_p1('Right', true)
wait_frames(12)
capture('04_walk.png')
clear_p1()
wait_frames(4)

tap_p1('Up', 2)
wait_frames(4)
capture('05_jump.png')
wait_frames(72)

tap_p1('A', 2)
wait_frames(2)
capture('06_jab.png')
wait_frames(12)

tap_p1('B', 2)
wait_frames(2)
capture('07_special.png')

finish(true, 'captured title, lobby, battle, walk, jump, jab, and special states')
