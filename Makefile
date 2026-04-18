.DEFAULT_GOAL := build

# ── Build ────────────────────────────────────────────────────────────────────

.PHONY: build
build:
	cargo build

.PHONY: release
release:
	cargo build --release

# ── Lint & Format ─────────────────────────────────────────────────────────────

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: fmt-check
fmt-check:
	cargo fmt -- --check

.PHONY: clippy
clippy:
	cargo clippy -- -D warnings

# ── Test ─────────────────────────────────────────────────────────────────────

.PHONY: test
test:
	cargo test

.PHONY: test-verbose
test-verbose:
	cargo test -- --nocapture

.PHONY: snapshots
snapshots:
	INSTA_UPDATE=always cargo test

# ── Combined checks (run before committing) ───────────────────────────────────

.PHONY: check
check: fmt-check clippy test

# ── Run ──────────────────────────────────────────────────────────────────────

.PHONY: run
run:
	cargo run -- examples/sequence.puml -o out.svg
	@echo "wrote out.svg"

# ── Clean ────────────────────────────────────────────────────────────────────

.PHONY: clean
clean:
	cargo clean
	rm -f out.svg
