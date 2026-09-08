local out_file = os.getenv('NESSY_TEST_OUT') or 'out/reports/fceux/boot_smoke.txt'
local base = 0x06e0

FCEU.speedmode('maximum')

for _ = 1, 180 do
  FCEU.frameadvance()
end

local signature_ok = memory.readbyte(base) == 0x53 and memory.readbyte(base + 1) == 0x4e
local frame_before = memory.readbyte(base + 4)

for _ = 1, 120 do
  FCEU.frameadvance()
end

local frame_after = memory.readbyte(base + 4)
local frame_delta = (frame_after - frame_before) % 256
local passed = signature_ok and frame_delta >= 100
local report = io.open(out_file, 'w')
if report then
  report:write((passed and 'PASS' or 'FAIL') .. '\n')
  report:write(string.format(
    'FCEUX NROM boot signature=%s frame=%d->%d delta=%d\n',
    signature_ok and 'SN' or 'bad',
    frame_before,
    frame_after,
    frame_delta
  ))
  report:close()
end

os.exit(passed and 0 or 1)
