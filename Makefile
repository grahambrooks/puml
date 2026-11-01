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

.PHONY: build-release
build-release: ## Optimised local build (cargo build --release)
	cargo build --release

# ── Release ──────────────────────────────────────────────────────────────────
# Pushing a vYYYY.M.D tag runs .github/workflows/release.yml, which stamps the
# version into Cargo.toml, builds and attaches the binaries, and merges a PR
# updating Formula/puml.rb and the version. So no local version bump here.

CALVER := $(shell date -u +'%Y.%-m.%-d')

.PHONY: release-version
release-version: ## Print the calver tag `make release` would push today
	@echo v$(CALVER)

.PHONY: release
release: ## Cut a calver release: check, tag v<today>, push (triggers the release workflow)
	@set -e; \
	v='$(CALVER)'; \
	echo "==> Cutting release v$$v"; \
	if ! git diff --quiet HEAD || [ -n "$$(git ls-files --others --exclude-standard)" ]; then \
		echo "error: working tree has uncommitted or untracked changes; commit or stash first." >&2; \
		exit 1; \
	fi; \
	branch="$$(git symbolic-ref --short HEAD)"; \
	if [ "$$branch" != "main" ]; then \
		echo "error: not on main (current branch: $$branch); releases are cut from main." >&2; \
		exit 1; \
	fi; \
	git fetch --quiet origin main --tags; \
	if [ "$$(git rev-parse HEAD)" != "$$(git rev-parse origin/main)" ]; then \
		echo "error: main differs from origin/main; pull or push first." >&2; \
		exit 1; \
	fi; \
	if git rev-parse -q --verify "refs/tags/v$$v" >/dev/null; then \
		echo "error: tag v$$v already exists. Delete it first or wait until tomorrow." >&2; \
		exit 1; \
	fi; \
	echo "==> Running checks"; \
	$(MAKE) --no-print-directory check; \
	echo "==> Tagging and pushing v$$v (this triggers the release workflow)"; \
	git tag -a "v$$v" -m "puml v$$v"; \
	git push origin "v$$v"; \
	echo "==> Release v$$v triggered. The workflow merges a PR bumping Cargo.toml"; \
	echo "    and Formula/puml.rb; pull main afterwards."; \
	echo "    Watch: https://github.com/grahambrooks/puml/actions"

# ── Lint & Format ─────────────────────────────────────────────────────────────

.PHONY: fmt
fmt: ## Format sources
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Verify formatting without writing
	cargo fmt -- --check

.PHONY: clippy
clippy: ## Lint with clippy (warnings as errors)
	cargo clippy --all-targets -- -D warnings

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
