.DEFAULT_GOAL := help

# ── Help ─────────────────────────────────────────────────────────────────────

.PHONY: help
help:
	@echo "Available targets:"
	@grep -hE '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  %-16s %s\n", $$1, $$2}'

# ── Build ────────────────────────────────────────────────────────────────────

.PHONY: build
build: fmt lint ## Debug build (runs fmt and clippy first)
	cargo build

.PHONY: build-release
build-release: ## Optimised local build (cargo build --release)
	cargo build --release

# ── Release ──────────────────────────────────────────────────────────────────
# release-kit's release.mk: `make release` tags origin/main with the next CalVer and
# pushes only the tag; .github/workflows/release.yml builds, publishes, and merges a
# PR bumping Cargo.toml and Formula/puml.rb. `make release-dry` previews it.

include release.mk

# ── Lint & Format ─────────────────────────────────────────────────────────────

.PHONY: fmt
fmt: ## Format sources
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting without writing
	cargo fmt --all --check

.PHONY: lint
lint: ## Lint with clippy (warnings as errors), as CI does
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

.PHONY: clippy
clippy: lint ## Alias for lint

# ── Test ─────────────────────────────────────────────────────────────────────

.PHONY: test
test: ## Run tests, as CI does
	cargo test --workspace --all-features --locked

.PHONY: test-verbose
test-verbose: ## Run tests with stdout captured
	cargo test -- --nocapture

.PHONY: snapshots
snapshots: ## Update insta snapshots
	INSTA_UPDATE=always cargo test

# ── Combined checks (run before committing) ───────────────────────────────────

.PHONY: check
check: fmt-check lint test ## The gate: fmt-check + clippy + test, exactly what CI's check job runs

.PHONY: hooks
hooks: ## Install the git hooks (pre-commit and pre-push) with prek
	prek install

# ── Run ──────────────────────────────────────────────────────────────────────

.PHONY: run
run: ## Render examples/sequence.puml to out.svg
	cargo run -- examples/sequence.puml -o out.svg
	@echo "wrote out.svg"

.PHONY: png
png: ## Render every examples/*.puml into docs/examples/*.png (debug renders)
	@cargo build --quiet
	@mkdir -p docs/examples
	@for f in examples/*.puml; do \
		name=$$(basename "$$f" .puml); \
		./target/debug/puml "$$f" -o "docs/examples/$$name.png" \
			&& echo "  wrote docs/examples/$$name.png" \
			|| { echo "  FAIL  $$f"; exit 1; }; \
	done

# ── Docs ─────────────────────────────────────────────────────────────────────

.PHONY: docs
docs: ## Re-render every examples/*.puml into docs/examples/*.svg
	@cargo build --quiet
	@mkdir -p docs/examples
	@for f in examples/*.puml; do \
		name=$$(basename "$$f" .puml); \
		./target/debug/puml "$$f" -o "docs/examples/$$name.svg" \
			&& echo "  wrote docs/examples/$$name.svg" \
			|| { echo "  FAIL  $$f"; exit 1; }; \
	done

# ── Clean ────────────────────────────────────────────────────────────────────

.PHONY: clean
clean: ## Remove build artifacts and out.svg
	cargo clean
	rm -f out.svg
