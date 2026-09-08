ROOT := $(abspath .)
CLI := cargo run --package nessy --
EMULATOR ?= mame

.PHONY: help doctor rebuild-fighters capture-kung-fu-reference rebuild-music check-music generate build inspect run capture capture-audio capture-theme title-iterations smoke tas test-emu test-fceux test-web test fmt web-sync web-dev ci-pages clean

help:
	@echo "Targets:"
	@echo "  make doctor     # check required/optional toolchain binaries"
	@echo "  make rebuild-fighters # redownload/verify sources and rebuild fighter sheets"
	@echo "  make capture-kung-fu-reference # TAS a locally owned original ROM for comparison"
	@echo "  make rebuild-music # verify the locked MIDI and rebuild the NES reduction"
	@echo "  make check-music # verify the tracked score against the locked MIDI offline"
	@echo "  make generate   # build all character assets from src/chars/*/character.toml"
	@echo "  make build      # generate assets + build ROM + inspect iNES header"
	@echo "  make inspect    # inspect built ROM metadata"
	@echo "  make run        # build ROM and launch emulator (EMULATOR=mame|fceux)"
	@echo "  make capture    # drive gameplay and validate native 256x240 screenshots"
	@echo "  make capture-audio # capture the same drive with a validated WAV"
	@echo "  make capture-theme # record the full Menu 1 loop from the NES APU"
	@echo "  make title-iterations # build and capture three native title directions"
	@echo "  make smoke      # run single boot smoke emulator case"
	@echo "  make tas        # run four per-fighter deterministic animation drives"
	@echo "  make test-emu   # run full headless emulator suite"
	@echo "  make test-fceux # verify the ROM in an independent native emulator"
	@echo "  make test-web   # build + verify + screenshot web UI headlessly"
	@echo "  make test       # run rust tests + emulator suite + web suite"
	@echo "  make fmt        # format rust sources"
	@echo "  make web-sync   # copy ROM/docs/screens into web/public"
	@echo "  make web-dev    # serve the synced static site on port 8080"
	@echo "  make ci-pages   # mirror GitHub Pages build and stage ./site artifact"
	@echo "  make clean      # remove generated outputs"

doctor:
	$(CLI) doctor

rebuild-fighters:
	nessy/tools/assets/rebuild_fighters.sh
	$(CLI) generate all

capture-kung-fu-reference:
	nessy/tools/assets/capture_kung_fu_reference.sh

rebuild-music:
	uv run nessy/tools/music/rebuild_music.py
	$(CLI) generate all

check-music:
	uv run nessy/tools/music/rebuild_music.py --offline --check

generate:
	$(CLI) generate all

build: generate
	$(CLI) rom build
	$(CLI) rom inspect --rom out/smash.nes

inspect:
	$(CLI) rom inspect --rom out/smash.nes

run: build
	@if [ "$(EMULATOR)" = "mame" ]; then \
		mame nes -cart out/smash.nes; \
	elif [ "$(EMULATOR)" = "fceux" ]; then \
		fceux out/smash.nes; \
	else \
		echo "Unsupported EMULATOR='$(EMULATOR)'. Use mame or fceux."; \
		exit 1; \
	fi

capture: build
	$(CLI) emulator capture

capture-audio: build
	$(CLI) emulator capture --audio-output out/reports/emulator/smash_audio.wav

capture-theme: build
	mkdir -p $(ROOT)/out/reports/emulator
	rm -f $(ROOT)/out/reports/emulator/melee_theme.wav $(ROOT)/out/reports/emulator/theme_preview.txt
	$(CLI) emulator run --backend mame --rom out/smash.nes \
		--script nessy/tools/emulator/theme_preview.lua --timeout-seconds 80 --audio -- \
		-wavwrite $(ROOT)/out/reports/emulator/melee_theme.wav
	rg -q '^PASS$$' $(ROOT)/out/reports/emulator/theme_preview.txt

title-iterations: generate
	@set -eu; \
	for variant in world challengers emblem; do \
		dir="out/reports/title-iterations/$$variant"; \
		mkdir -p "$$dir"; \
		$(CLI) rom build --skip-generate --title-variant "$$variant" \
			--output "$$dir/smash.nes" --map "$$dir/smash.map" --labels "$$dir/smash.lbl"; \
		$(CLI) emulator capture --rom "$$dir/smash.nes" \
			--output-dir "$$dir/snaps" --report "$$dir/capture.txt"; \
		$(CLI) test emulator --backend mame --rom "$$dir/smash.nes" \
			--report-dir "$$dir/cadence" --case battle_cadence; \
	done
	magick \
		out/reports/title-iterations/world/snaps/01_title.png \
		out/reports/title-iterations/challengers/snaps/01_title.png \
		out/reports/title-iterations/emblem/snaps/01_title.png \
		+append \
		out/reports/title-iterations/contact-sheet.png
	$(CLI) rom build --skip-generate --title-variant world
	$(CLI) rom inspect --rom out/smash.nes

smoke: build
	$(CLI) test emulator --backend mame --rom out/smash.nes --case boot_smoke

tas: build
	$(CLI) test emulator --backend mame --rom out/smash.nes \
		--case tas_mario --case tas_simon --case tas_tommy --case tas_rizer
	nessy/tools/emulator/build_tas_contact_sheets.sh

test-emu: build
	$(CLI) test emulator --backend mame --rom out/smash.nes

test-fceux: build
	$(CLI) test emulator --backend fceux --rom out/smash.nes \
		--suite-dir nessy/tools/emulator/fceux \
		--report-dir out/reports/fceux

test-web:
	nessy/tools/web/test_web.sh

test:
	$(MAKE) check-music
	cargo nextest run --workspace
	cargo test --workspace --doc
	$(MAKE) test-emu
	$(MAKE) test-web

fmt:
	cargo fmt --all

web-sync: build
	$(CLI) web sync

web-dev: web-sync
	uv run python -m http.server 8080 --bind 127.0.0.1 --directory web/public

ci-pages: web-sync
	rm -rf $(ROOT)/site
	mkdir -p $(ROOT)/site
	cp -R $(ROOT)/web/public/. $(ROOT)/site/
	diff -qr $(ROOT)/web/public $(ROOT)/site

clean:
	rm -rf out web/public/rom/smash.nes web/public/snaps web/public/docs web/public/info/build.json src/generated src/chars/*/generated cfg nvram
	mkdir -p web/public/snaps src/generated
