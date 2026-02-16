Static assets for the publish web build.

`emulatorjs/data/` contains vendored EmulatorJS runtime files and NES cores so the
browser emulator can run without any runtime CDN/network dependency.

This directory also ensures the generated web bundle always contains an `assets/`
directory, which avoids intermittent `dx build` copy-path failures for top-level
public files.
