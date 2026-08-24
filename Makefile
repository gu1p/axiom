CARGO ?= cargo
CARGO_HOME ?= $(HOME)/.cargo
INSTALL_ROOT ?= $(CARGO_HOME)
INSTALL_BIN_DIR := $(INSTALL_ROOT)/bin
PROFILE ?=

.DEFAULT_GOAL := build
.NOTPARALLEL:
.PHONY: build install check fmt-check lint test semantic-driver policy-check installer-check

build:
	$(CARGO) build --workspace --all-targets --locked

install:
	$(CARGO) install --path crates/cargo-policy --root "$(INSTALL_ROOT)" --locked --force --bin axiom --bin cargo-policy
	$(CARGO) install --path crates/policy-semantic --root "$(INSTALL_ROOT)" --locked --force --no-default-features --features driver --bin axiom-hir-driver
	@profile='$(PROFILE)'; \
	if [ -z "$$profile" ]; then \
		case "$${SHELL##*/}" in \
			zsh) profile="$$HOME/.zshrc" ;; \
			bash) \
				if [ "$$(uname -s)" = "Darwin" ]; then \
					profile="$$HOME/.bash_profile"; \
				else \
					profile="$$HOME/.bashrc"; \
				fi ;; \
			fish) profile="$$HOME/.config/fish/config.fish" ;; \
			*) profile="$$HOME/.profile" ;; \
		esac; \
	fi; \
	mkdir -p "$$(dirname "$$profile")"; \
	touch "$$profile"; \
	if [ "$${SHELL##*/}" = "fish" ]; then \
		line='fish_add_path "$(INSTALL_BIN_DIR)"'; \
	else \
		line='export PATH="$(INSTALL_BIN_DIR):$$PATH"'; \
	fi; \
	if ! grep -Fqx "$$line" "$$profile"; then \
		printf '\n%s\n' "$$line" >> "$$profile"; \
		echo "Added $(INSTALL_BIN_DIR) to PATH in $$profile"; \
	else \
		echo "$(INSTALL_BIN_DIR) is already configured in $$profile"; \
	fi; \
	echo "Installed axiom, cargo-policy, and Axiom's semantic driver in $(INSTALL_BIN_DIR)"; \
	echo "Restart your shell to load the updated PATH."

check: fmt-check lint test policy-check installer-check

fmt-check:
	$(CARGO) fmt --all --check

lint:
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

test: semantic-driver
	$(CARGO) test --workspace --locked

semantic-driver:
	$(CARGO) build --locked -p policy-semantic --bin axiom-hir-driver

policy-check: semantic-driver
	$(CARGO) run --locked -p cargo-policy -- check

installer-check:
	sh -n install.sh
