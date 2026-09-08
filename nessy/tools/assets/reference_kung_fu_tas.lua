local artifact_dir = os.getenv('KUNG_FU_CAPTURE_DIR') or 'out/reference-kung-fu'
local report_file = os.getenv('KUNG_FU_CAPTURE_REPORT') or
    'out/reference-kung-fu/report.txt'

local screen = manager.machine.screens[':screen']
local port = manager.machine.ioport.ports[':ctrl1:joypad:JOYPAD']
local p1 = {}
for _, field in pairs(port.fields) do
  local name = tostring(field.name)
  if string.sub(name, 1, 3) == 'P1 ' and p1[name] == nil then
    p1[name] = field
  end
end

local screenshot_count = 0

local function wait_frames(count)
  for _ = 1, count do
    emu.wait_next_frame()
  end
end

local function clear_inputs()
  for _, field in pairs(p1) do
    field:set_value(0)
  end
end

local function set_button(button, down)
  local field = p1['P1 ' .. button]
  assert(field ~= nil, 'missing P1 ' .. button)
  field:set_value(down and 1 or 0)
end

local function tap(button, frames)
  clear_inputs()
  set_button(button, true)
  wait_frames(frames or 2)
  clear_inputs()
  wait_frames(2)
end

local function snapshot(name)
  screen:snapshot(artifact_dir .. '/' .. name)
  screenshot_count = screenshot_count + 1
end

local function capture_timeline(prefix, frames, stride)
  for index = 0, frames - 1 do
    wait_frames(stride)
    snapshot(string.format('%s-%02d.png', prefix, index))
  end
end

local function finish(ok, detail)
  clear_inputs()
  local report = io.open(report_file, 'w')
  if report then
    report:write((ok and 'PASS' or 'FAIL') .. '\n')
    report:write(detail .. '\n')
    report:close()
  end
  manager.machine:exit()
end

wait_frames(240)
snapshot('00-title.png')
tap('Start', 2)
wait_frames(300)
snapshot('01-stage-idle.png')

set_button('Right', true)
capture_timeline('02-walk', 8, 3)
clear_inputs()
wait_frames(24)

set_button('Right', true)
set_button('Up', true)
wait_frames(2)
set_button('Up', false)
capture_timeline('03-jump', 12, 3)
clear_inputs()
wait_frames(90)

set_button('A', true)
wait_frames(1)
set_button('A', false)
capture_timeline('04-punch', 10, 1)
wait_frames(24)

set_button('B', true)
wait_frames(1)
set_button('B', false)
capture_timeline('05-kick', 12, 1)

finish(true, string.format(
  'captured %d native Kung Fu reference frames: title, stage, walk, jump, punch, kick',
  screenshot_count
))
