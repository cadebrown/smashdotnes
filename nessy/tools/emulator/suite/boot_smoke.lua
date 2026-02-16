-- Legacy-compatible boot smoke test.
-- Expects test export layout used by original runtime at $06E0.

local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/boot_smoke.txt'

local cpu = manager.machine.devices[':maincpu']
local prg = cpu.spaces['program']
local BASE = 0x06E0

local function rb(off)
  return prg:read_u8(BASE + off)
end

local function wait_frames(n)
  for _ = 1, n do
    emu.wait_next_frame()
  end
end

local function write_result(ok, message)
  local f = io.open(out_file, 'w')
  if f then
    f:write((ok and 'PASS' or 'FAIL') .. '\n')
    f:write(message .. '\n')
    f:close()
  end
end

wait_frames(120)

if rb(0) ~= 0x53 or rb(1) ~= 0x4E then
  write_result(false, 'missing SN signature at $06E0')
  manager.machine:exit()
  return
end

if rb(2) ~= 0 then
  write_result(false, string.format('expected TITLE state (0), got %d', rb(2)))
  manager.machine:exit()
  return
end

write_result(true, 'boot signature and TITLE state verified')
manager.machine:exit()
