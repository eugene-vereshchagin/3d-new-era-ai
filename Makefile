# 3D New Era AI — development shortcuts.
# `make` on its own lists everything.

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

CARGO   ?= cargo
ADDR    ?= 127.0.0.1:7878
BIN     := target/debug/newera
RELEASE := target/release/newera
LOG     ?= info,wgpu_core=warn,wgpu_hal=warn,naga=warn,rmcp=warn,egui_wgpu=warn

# Local secrets — the Sentry DSN — live in a git-ignored .env.local, so a
# build from this machine reports and the repository never carries them. The
# usage count needs none: it goes through the site's own endpoint (and
# NEWERA_PING_URL points it at a local one).
-include .env.local
export NEWERA_SENTRY_DSN

export NEWERA_ADDR := $(ADDR)
export NEWERA_LOG  := $(LOG)

BOLD  := \033[1m
CYAN  := \033[36m
DIM   := \033[2m
RESET := \033[0m

##@ Run

# The project to open: the sample house, or FILE=plan.newera — the one with the
# bug you are chasing, reopened by `make dev` after every rebuild.
FILE ?=
OPEN := $(if $(FILE),$(FILE),--demo)

# The build to iterate on: optimized without LTO, so the editor is both quick to
# rebuild and quick to use (Cargo.toml says what it measures). `PROFILE=dev` for
# a debugger, full debug info and an unoptimized build.
PROFILE ?= quick

.PHONY: run
run: ## Opens the editor with the demo house (or FILE=plan.newera) + HTTP + MCP
	$(CARGO) run -p newera --profile $(PROFILE) -- $(OPEN)

.PHONY: run-empty
run-empty: ## Opens the editor with an empty project
	$(CARGO) run -p newera --profile $(PROFILE)

.PHONY: run-release
run-release: ## Opens the editor exactly as it ships (thin LTO; slowest to build)
	$(CARGO) run -p newera --release -- $(OPEN)

.PHONY: dev
dev: ## Rebuilds and reopens the editor on every change (cargo-watch)
	@command -v cargo-watch >/dev/null || { echo "cargo-watch missing: run 'make setup'"; exit 1; }
	cargo watch -c -w crates -x "run -p newera --profile $(PROFILE) -- $(OPEN)"

# A change in the core rebuilds every crate above it and relinks the editor:
# about forty seconds. Stopping at newera-render is a third of that, and for
# anything you have to *look* at — floors, walls, joins, materials — the
# picture is the answer, not the window.
SHOT_FILE ?= $(if $(FILE),$(FILE),web/demo.newera)
SHOT_OUT  ?= target/shot.png
SHOT_VIEW ?= aerial

.PHONY: shot
shot: ## Renders a project to a PNG, no window: make shot SHOT_VIEW='cam=2'
	@cargo run -q -p newera-render --example shot -- \
	  $(SHOT_FILE) $(SHOT_OUT) $(SHOT_VIEW)

.PHONY: watch
watch: ## Re-renders that PNG on every change (cargo-watch)
	@command -v cargo-watch >/dev/null || { echo "cargo-watch missing: run 'make setup'"; exit 1; }
	cargo watch -c -w crates -x "run -q -p newera-render --example shot -- \
	  $(SHOT_FILE) $(SHOT_OUT) $(SHOT_VIEW)"

.PHONY: serve
serve: ## Only the HTTP + MCP server, no window (headless)
	$(CARGO) run -p newera -- serve --demo

.PHONY: mcp-stdio
mcp-stdio: ## MCP over stdin/stdout (for clients that start the process)
	$(CARGO) run -q -p newera -- mcp --demo

.PHONY: icons
icons: ## Regenerates every icon (app, document, macOS, Windows, Linux) from the mark
	python3 scripts/make-icons.py

.PHONY: logo
logo: ## Prints the mark in ANSI (the one the installers show)
	@cat assets/logo.ansi

.PHONY: web
web: ## Builds the web viewer (WebAssembly) into web/
	@rustup target list --installed | grep -q wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
	$(CARGO) build -p newera-web --release --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/release/newera_web.wasm web/

.PHONY: web-editor
web-editor: ## Builds the full browser editor into web/editor/
	@rustup target list --installed | grep -q wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
	@command -v wasm-bindgen >/dev/null || cargo install wasm-bindgen-cli --version 0.2.128 --locked
	$(CARGO) build -p newera-editor-web --release --target wasm32-unknown-unknown
	wasm-bindgen --target web --no-typescript --out-dir web/editor/pkg target/wasm32-unknown-unknown/release/newera_editor_web.wasm
	rm -rf web/editor/assets && mkdir -p web/editor/assets && cp -r crates/newera-catalog/assets/seating web/editor/assets/

.PHONY: web-serve
web-serve: web web-editor ## Serves the viewer (/) and the editor (/editor/) at http://127.0.0.1:8790
	python3 -m http.server 8790 --bind 127.0.0.1 --directory web

##@ Quality

.PHONY: check
check: fmt-check lint test smoke ## Everything CI runs: fmt, clippy, tests and the MCP smoke test

.PHONY: fmt
fmt: ## Formats the code
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Checks formatting without changing files
	$(CARGO) fmt --all -- --check

.PHONY: lint
lint: ## Clippy pedantic, warnings as errors
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

# The pages the spell checker reads, named rather than globbed: a path given on
# the command line is checked whether _typos.toml excludes it or not, and
# README.pt-BR.md is Portuguese. Keep in step with .github/workflows/docs.yml.
DOCS_PATHS := README.md CHANGELOG.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md docs .github/ISSUE_TEMPLATE .github/pull_request_template.md plugin plugin-desktop

.PHONY: docs-lint
docs-lint: ## What the Docs workflow runs: Markdown rules, spelling and relative links
	npx --yes markdownlint-cli2
	@if command -v typos >/dev/null; then \
		typos $(DOCS_PATHS); \
	else \
		echo "typos not installed, skipping (brew install typos-cli)"; \
	fi
	@if command -v lychee >/dev/null; then \
		lychee --offline --no-progress "**/*.md"; \
	else \
		echo "lychee not installed, skipping (brew install lychee)"; \
	fi

.PHONY: test
test: ## Unit and integration tests
	$(CARGO) test --workspace --locked

.PHONY: golden
golden: ## Every review finding over a fixed corpus; UPDATE=1 rewrites the snapshot
	$(if $(UPDATE),NEWERA_UPDATE_GOLDEN=1) $(CARGO) test --profile quick -p newera-ergonomics --test golden --locked -- --ignored

.PHONY: smoke
smoke: build ## Starts the server and talks MCP the way an AI would
	scripts/mcp-smoke.sh $(BIN)

.PHONY: deny
deny: ## Audits dependency licenses and advisories (cargo-deny)
	cargo deny check

.PHONY: fix
fix: ## Applies clippy's automatic fixes and formats
	$(CARGO) clippy --workspace --all-targets --fix --allow-dirty --allow-staged
	$(CARGO) fmt --all

##@ Build

.PHONY: build
build: ## Debug build
	$(CARGO) build -p newera

.PHONY: release
release: ## Optimized build at target/release/newera
	$(CARGO) build -p newera --release
	@ls -lh $(RELEASE)

.PHONY: doc
doc: ## Builds and opens the crate documentation
	$(CARGO) doc --workspace --no-deps --open

.PHONY: clean
clean: ## Removes build artifacts
	$(CARGO) clean

##@ Install on this system

APPS ?= $(HOME)/Applications

.PHONY: install
install: ## Installs the app from this checkout (macOS: ~/Applications; Linux: menu, icons and .newera)
	@set -e; \
	case "$$(uname -s)" in \
	Darwin) \
	  $(CARGO) build -p newera --release; \
	  app=$$(scripts/macos-app.sh "$(RELEASE)" "$(APPS)" "$$(git rev-parse HEAD 2>/dev/null || true)" | tail -1); \
	  reg=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister; \
	  [ -x $$reg ] && $$reg -f "$$app" >/dev/null 2>&1 || true; \
	  mkdir -p "$(HOME)/.local/bin"; \
	  ln -sf "$$app/Contents/MacOS/newera" "$(HOME)/.local/bin/newera"; \
	  echo "instalado: $$app"; \
	  ;; \
	Linux) \
	  $(CARGO) build -p newera --release; \
	  stage=target/desktop; \
	  rm -rf $$stage; \
	  mkdir -p $$stage/share/applications $$stage/share/mime/packages $$stage/share/metainfo $$stage/share/icons; \
	  cp $(RELEASE) scripts/install-desktop.sh $$stage/; \
	  cp assets/linux/newera.desktop $$stage/share/applications/; \
	  cp assets/linux/newera.xml $$stage/share/mime/packages/; \
	  cp assets/linux/io.github.leandrodaf.newera.metainfo.xml $$stage/share/metainfo/; \
	  cp -r assets/linux/hicolor $$stage/share/icons/; \
	  sh $$stage/install-desktop.sh; \
	  ;; \
	*) echo "no Windows: scripts/install-windows.ps1"; exit 1;; \
	esac

.PHONY: uninstall
uninstall: ## Removes what `make install` put on the system
	@case "$$(uname -s)" in \
	Darwin) \
	  rm -rf "$(APPS)/3D New Era AI.app"; \
	  rm -f "$(HOME)/.local/bin/newera"; \
	  echo "removidos: o app e o link newera"; \
	  ;; \
	Linux) sh scripts/install-desktop.sh --uninstall;; \
	*) echo "no Windows: scripts/install-windows.ps1 -Uninstall"; exit 1;; \
	esac

##@ AI / MCP

.PHONY: mcp-add-claude
mcp-add-claude: ## Registers the editor's MCP in Claude Code (with the editor open)
	claude mcp add --transport http newera http://$(ADDR)/mcp

.PHONY: mcp-tools
mcp-tools: ## Lists the MCP tools (needs the editor or server running)
	@H=(-H "Content-Type: application/json" -H "Accept: application/json, text/event-stream"); \
	curl -s "$${H[@]}" -D /tmp/newera-mcp.h http://$(ADDR)/mcp \
	  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"make","version":"1"}}}' >/dev/null; \
	S=$$(grep -i mcp-session-id /tmp/newera-mcp.h | awk '{print $$2}' | tr -d '\r'); \
	H+=(-H "Mcp-Session-Id: $$S" -H "MCP-Protocol-Version: 2025-06-18"); \
	curl -s "$${H[@]}" http://$(ADDR)/mcp -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' >/dev/null; \
	curl -s "$${H[@]}" http://$(ADDR)/mcp -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
	  | sed -n 's/^data: //p' \
	  | python3 -c 'import json,sys; [print("  " + t["name"] + " — " + t.get("description", "")) for t in json.load(sys.stdin)["result"]["tools"]]'

.PHONY: mcp
mcp: ## Calls an MCP tool: make mcp TOOL=get_home ARGS='{}'
	@scripts/mcp.sh $(TOOL) '$(or $(ARGS),{})'

.PHONY: home
home: ## Prints the open home as JSON (GET /api/home)
	@curl -s http://$(ADDR)/api/home | python3 -m json.tool

##@ Environment

.PHONY: setup
setup: ## Installs the development components and tools
	rustup component add rustfmt clippy
	$(CARGO) install --locked cargo-watch cargo-deny
	@if [ "$$(uname)" = "Linux" ]; then \
	  echo -e "\n$(DIM)Dependências de sistema (Debian/Ubuntu), se faltar alguma:$(RESET)"; \
	  echo "  sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libvulkan1"; \
	fi

.PHONY: help
help: ## Lists the commands
	@awk 'BEGIN {FS = ":.*##"; printf "\n$(BOLD)3D New Era AI$(RESET)  $(DIM)make <target> [ADDR=127.0.0.1:7878]$(RESET)\n"} \
	  /^[a-zA-Z_-]+:.*?##/ { printf "  $(CYAN)%-16s$(RESET) %s\n", $$1, $$2 } \
	  /^##@/ { printf "\n$(BOLD)%s$(RESET)\n", substr($$0, 5) }' $(MAKEFILE_LIST)
	@echo
