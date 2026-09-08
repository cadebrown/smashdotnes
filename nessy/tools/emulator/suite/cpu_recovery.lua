local h = dofile('nessy/tools/emulator/suite/quality_common.lua')

h.run(function()
  h.start(1)
  -- Tommy's CPU has no projectile, so the bait tests movement and recovery.
  h.tap(1, 'Down')
  h.battle()
  assert(h.rb(3) == 1, 'expected human versus CPU mode')
  h.pad(1, {'Right'})
  local previous_x, previous_y
  assert(h.until_true(function()
    if h.rs16(14) >= 224 then return true end
    previous_x, previous_y = h.rs16(14), h.rs16(16)
    return false
  end, 180, 'bait'), 'CPU did not walk offstage toward the human bait')
  h.pad(1)
  h.snapshot('01-bait')
  local departure_x, departure_y = h.rs16(14), h.rs16(16)
  -- Grounded immediately before departure proves jumps were replenished;
  -- this must exercise a walk-off recovery, not an existing aerial jump.
  assert(previous_x == 222 and previous_y == 143 and departure_y == 143,
    string.format('expected grounded walk-off: previous=(%s,%s) departure=(%d,%d)',
      tostring(previous_x), tostring(previous_y), departure_x, departure_y))
  local kos = h.rb(13)
  h.snapshot('02-cpu-offstage')
  assert(h.until_true(function()
    return h.rs16(14) < departure_x and h.rs16(16) < departure_y
  end, 36, 'cpu-recovery'), string.format('CPU did not jump inward after walking off: departure=(%d,%d) current=(%d,%d) kos=%d->%d', departure_x, departure_y, h.rs16(14), h.rs16(16), kos, h.rb(13)))
  assert(h.rb(13) == kos, 'CPU respawned instead of recovering')
  h.tick(2, 'visible-recovery')
  h.snapshot('03-cpu-return')
  -- Bring the human back so the recovered CPU can settle onstage.
  h.pad(1, {'Left'})
  assert(h.until_true(function() return h.rs16(8) < 184 end, 80, 'human-return'), 'human could not return')
  h.pad(1)
  assert(h.until_true(function()
    return h.rs16(14) > 16 and h.rs16(14) < 224 and h.rs16(16) == 143
  end, 100, 'cpu-land'), 'CPU did not return to the platform')
  assert(h.rb(13) == kos, 'CPU lost a stock during recovery')
  h.snapshot('04-cpu-landed')
  return string.format('CPU walked offstage at (%d,%d), jumped inward, and landed without a KO', departure_x, departure_y)
end)
