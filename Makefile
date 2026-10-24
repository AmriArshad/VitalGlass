# Build and install VitalGlass for the current user. No root needed.
#
#   make            release build (target/release/vitalglass)
#   make install    build, then install into ~/.local (binary, icons, launcher entry)
#   make uninstall  remove everything install put there (not your data or sign-in)
#   make check      lint and test everything
#
# Override the location with PREFIX, e.g. `make install PREFIX=/opt/vitalglass`.

PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin
APPDIR := $(PREFIX)/share/applications
ICONDIR := $(PREFIX)/share/icons/hicolor
APP_ID := io.github.amriarshad.vitalglass
BINARY := target/release/vitalglass

# Icon sizes shipped in src-tauri/icons, as "size:source file".
ICONS := 32:32x32.png 64:64x64.png 128:128x128.png 256:128x128@2x.png

.PHONY: all build install uninstall check

all: build

build: node_modules
	npx tauri build --no-bundle

# Install JS dependencies only when the lockfile changes.
node_modules: package-lock.json
	npm ci
	@touch node_modules

install: build
	install -Dm755 $(BINARY) $(BINDIR)/vitalglass
	@for pair in $(ICONS); do \
		size=$${pair%%:*}; file=$${pair#*:}; \
		install -Dm644 "src-tauri/icons/$$file" "$(ICONDIR)/$${size}x$${size}/apps/vitalglass.png"; \
	done
	install -Dm644 src-tauri/icons/icon.svg $(ICONDIR)/scalable/apps/vitalglass.svg
	@mkdir -p $(APPDIR)
	sed 's|@BINDIR@|$(BINDIR)|' packaging/vitalglass.desktop > $(APPDIR)/$(APP_ID).desktop
	@chmod 644 $(APPDIR)/$(APP_ID).desktop
	@# Refresh the launcher caches; harmless if a tool is missing.
	-@update-desktop-database -q $(APPDIR) 2>/dev/null
	-@kbuildsycoca6 >/dev/null 2>&1
	@echo "Installed. Launch VitalGlass from the app menu or run $(BINDIR)/vitalglass."

uninstall:
	rm -f $(BINDIR)/vitalglass $(APPDIR)/$(APP_ID).desktop $(ICONDIR)/scalable/apps/vitalglass.svg
	@for pair in $(ICONS); do \
		size=$${pair%%:*}; rm -f "$(ICONDIR)/$${size}x$${size}/apps/vitalglass.png"; \
	done
	-@update-desktop-database -q $(APPDIR) 2>/dev/null
	-@kbuildsycoca6 >/dev/null 2>&1
	@echo "Removed. Your cache (~/.local/share/vitalglass) and keyring sign-in are untouched."

check: node_modules
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	npm run check
