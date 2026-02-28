SHELL := /bin/bash
.DEFAULT_GOAL := help

EXT_DIR ?= editors/vscode-dtl
CURSOR_BIN ?= cursor

.PHONY: help package-extension install install-cursor

help:
	@echo "Targets:"
	@echo "  make package-extension   # Build VSIX for the DTL extension"
	@echo "  make install             # Build and install VSIX into Cursor"
	@echo "  make install-cursor      # Alias of make install"

package-extension:
	bun install --cwd $(EXT_DIR)
	bun run --cwd $(EXT_DIR) package

install: install-cursor

install-cursor: package-extension
	@command -v $(CURSOR_BIN) >/dev/null || { echo "error: '$(CURSOR_BIN)' command not found in PATH"; exit 1; }
	@vsix_path="$$(ls -1t $(EXT_DIR)/dtl-*.vsix 2>/dev/null | head -n1)"; \
	if [ -z "$$vsix_path" ]; then \
		echo "error: VSIX not found under $(EXT_DIR)"; \
		exit 1; \
	fi; \
	log_file="$$(mktemp)"; \
	$(CURSOR_BIN) --install-extension "$$vsix_path" 2>&1 | tee "$$log_file"; \
	if grep -Eq "Failed Installing Extensions|Unable to install extension" "$$log_file"; then \
		rm -f "$$log_file"; \
		echo "error: Cursor rejected extension install"; \
		exit 1; \
	fi; \
	rm -f "$$log_file"; \
	echo "installed: $$vsix_path"
