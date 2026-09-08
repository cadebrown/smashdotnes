local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local base = 0x06E0
local report_path = 'out/reports/emulator/theme_preview.txt'

local function rb(offset)
  return prg:read_u8(base + offset)
end

local function music_step()
  return rb(21) | (rb(22) << 8)
end

local function wait_frames(count)
  for _ = 1, count do
    emu.wait_next_frame()
  end
end

local function finish(ok, detail)
  local report = io.open(report_path, 'w')
  if report then
    report:write((ok and 'PASS' or 'FAIL') .. '\n')
    report:write(detail .. '\n')
    report:close()
  end
  manager.machine:exit()
end

wait_frames(120)
if rb(0) ~= 0x53 or rb(1) ~= 0x4E or rb(20) ~= 0 then
  finish(false, 'title scene did not initialize before theme capture')
  return
end

local previous = music_step()
local saw_last_step = false
local saw_loop_step = false
for _ = 1, 3800 do
  wait_frames(1)
  local step = music_step()
  if step == 925 then
    saw_last_step = true
  elseif saw_last_step and previous == 925 and step == 124 then
    saw_loop_step = true
  end
  previous = step
end

if not saw_last_step or not saw_loop_step then
  finish(false, string.format('Menu 1 loop missing: last=%s loop=%s step=%d', tostring(saw_last_step), tostring(saw_loop_step), music_step()))
  return
end

finish(true, 'captured all 926 Menu 1 events and observed event 925 loop to event 124')
