.DEFAULT_GOAL := help

# ── Help ─────────────────────────────────────────────────────────────────────

.PHONY: help
help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  %-14s %s\n", $$1, $$2}'

# ── Build ────────────────────────────────────────────────────────────────────

.PHONY: build
build: fmt clippy ## Debug build (runs fmt and clippy first)
	cargo build

.PHONY: release
release: ## Release build
	cargo build --release

# ── Lint & Format ─────────────────────────────────────────────────────────────

.PHONY: fmt
fmt: ## Format sources
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Verify formatting without writing
	cargo fmt -- --check

.PHONY: clippy
clippy: ## Lint with clippy (warnings as errors)
	cargo clippy -- -D warnings

# ── Test ─────────────────────────────────────────────────────────────────────

.PHONY: test
test: ## Run tests
	cargo test

.PHONY: test-verbose
test-verbose: ## Run tests with stdout captured
	cargo test -- --nocapture

.PHONY: snapshots
snapshots: ## Update insta snapshots
	INSTA_UPDATE=always cargo test

# ── Combined checks (run before committing) ───────────────────────────────────

.PHONY: check
check: fmt-check clippy test ## Pre-commit checks (fmt-check + clippy + test)

# ── Run ──────────────────────────────────────────────────────────────────────

.PHONY: run
run: ## Render examples/sequence.puml to out.svg
	cargo run -- examples/sequence.puml -o out.svg
	@echo "wrote out.svg"

# ── Clean ────────────────────────────────────────────────────────────────────

.PHONY: clean
clean: ## Remove build artifacts and out.svg
	cargo clean
	rm -f out.svg
