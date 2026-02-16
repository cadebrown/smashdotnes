ROOT := $(abspath .)
CLI := cargo run --package nessy --
EMULATOR ?= mame

.PHONY: help doctor generate build inspect run smoke test-emu test-web test fmt web-sync web-dev clean

help:
	@echo "Targets:"
	@echo "  make doctor     # check required/optional toolchain binaries"
	@echo "  make generate   # build all character assets from src/chars/*/character.toml"
	@echo "  make build      # generate assets + build ROM + inspect iNES header"
	@echo "  make inspect    # inspect built ROM metadata"
	@echo "  make run        # build ROM and launch emulator (EMULATOR=mame|fceux)"
	@echo "  make smoke      # run single boot smoke emulator case"
	@echo "  make test-emu   # run full headless emulator suite"
	@echo "  make test-web   # build + verify + screenshot web UI headlessly"
	@echo "  make test       # run rust tests + emulator suite + web suite"
	@echo "  make fmt        # format rust sources"
	@echo "  make web-sync   # copy ROM/docs/screens into web/public"
	@echo "  make web-dev    # run Dioxus dev server"
	@echo "  make clean      # remove generated outputs"

doctor:
	$(CLI) doctor

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

smoke: build
	$(CLI) test emulator --backend mame --rom out/smash.nes --case boot_smoke

test-emu: build
	$(CLI) test emulator --backend mame --rom out/smash.nes

test-web:
	nessy/tools/web/test_web.sh

test:
	cargo test --workspace
	$(MAKE) test-emu
	$(MAKE) test-web

fmt:
	cargo fmt --all

web-sync: build
	$(CLI) web sync

web-dev: web-sync
	dx serve --platform web --package smash-web

clean:
	rm -rf out web/public/rom/smash.nes web/public/snaps web/public/docs web/public/info/build.json src/generated src/chars/*/generated cfg nvram
	mkdir -p web/public/snaps src/generated
