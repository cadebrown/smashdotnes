local h = dofile('nessy/tools/emulator/suite/quality_common.lua')

h.run(function()
  local failures = {}
  local observations = {}
  for _, side in ipairs({'left', 'right'}) do
    local outward = side == 'left' and 'Left' or 'Right'
    local inward = side == 'left' and 'Right' or 'Left'
    local function offstage()
      local x = h.rs16(8)
      return side == 'left' and x <= 16 or side == 'right' and x >= 224
    end
    for _, recovery_jump in ipairs({false, true}) do
      local label = side .. (recovery_jump and '-jump' or '-fall')
      h.start()
      h.battle()
      h.pad(1, {outward})
      assert(h.until_true(offstage, 100, label .. '-approach'), label .. ' failed to reach edge')
      h.pad(1)
      assert(h.until_true(function() return h.rs16(10) >= 170 end, 24, label .. '-drop'), label .. ' did not fall below platform')
      local origin_y = h.rs16(10)
      h.snapshot(label .. '-below')
      h.pad(1, recovery_jump and {inward, 'Up'} or {inward})
      assert(h.until_true(function() return not offstage() end, 8, label .. '-reenter'), label .. ' failed to move below platform')
      h.pad(1)
      local entry_y = h.rs16(10)
      h.snapshot(label .. '-entry')
      table.insert(observations, string.format('%s:%d->%d', label, origin_y, entry_y))
      if entry_y <= 143 then
        table.insert(failures, label .. ' snapped to platform from below at y=' .. entry_y)
      elseif recovery_jump then
        assert(entry_y < origin_y, label .. ' jump did not rise')
        assert(h.until_true(function() return h.rs16(10) < 143 end, 16, label .. '-rise'), label .. ' could not rise through platform')
        assert(h.until_true(function() return h.rs16(10) == 143 end, 90, label .. '-land'), label .. ' did not land from above')
        assert(h.rb(19) == 0, label .. ' respawned instead of landing')
        h.tick(6, label .. '-grounded')
        assert(h.rs16(10) == 143, label .. ' did not remain grounded')
        h.snapshot(label .. '-landed')
      else
        assert(h.until_true(function() return h.rb(19) == 1 end, 50, label .. '-fallthrough'), label .. ' did not fall to the blast zone')
        assert(h.rs16(10) == 143, label .. ' failed to respawn')
        h.snapshot(label .. '-respawn')
      end
    end
  end
  assert(#failures == 0, table.concat(failures, '; '))
  return 'both edges reject landing from below; recovery jumps pass through and land from above; ' .. table.concat(observations, '; ')
end)
