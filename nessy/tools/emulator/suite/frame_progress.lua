-- Ensures exported 8-bit frame counter advances in title state.

local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/emulator/frame_progress.txt'

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

local function wrapped_delta(before, after)
  local d = after - before
  if d < 0 then
    d = d + 256
  end
  return d
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

if rb(2) ~= 0 then
  write_result(false, 'expected TITLE state before frame progression sample')
  manager.machine:exit()
  return
end

local before = rb(4)
wait_frames(120)
local after = rb(4)
local delta = wrapped_delta(before, after)

if delta < 100 then
  write_result(false, string.format('frame counter progression too small: before=%d after=%d delta=%d', before, after, delta))
  manager.machine:exit()
  return
end

write_result(true, string.format('frame counter advanced: before=%d after=%d delta=%d', before, after, delta))
manager.machine:exit()
