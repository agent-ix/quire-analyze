# =============================================================================
# Quire Analyze Makefile
# =============================================================================

ifneq ($(filter ci,$(MAKECMDGOALS)),)
ifneq ($(strip $(MAKEFLAGS)),)
$(error local CI refuses non-empty MAKEFLAGS)
endif
ifneq ($(origin CARGO),undefined)
$(error local CI refuses a CARGO override)
endif
endif

override CARGO := cargo

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make fmt              - Format with rustfmt"
	@echo "  make fmt-check        - Verify formatting (CI gate)"
	@echo "  make lint             - Clippy with -D warnings"
	@echo "  make test             - cargo test"
	@echo "  make spec             - Quire-validate the specification and plan"
	@echo "  make msrv             - Test the locked graph with Rust 1.75"
	@echo "  make rustdoc          - Build warning-free public documentation"
	@echo "  make coverage         - Enforce the local line-coverage floor"
	@echo "  make verify-evidence  - Verify retained evidence checksums"
	@echo "  make build            - Release build"
	@echo "  make clean            - cargo clean"
	@echo "  make deny             - cargo deny check licenses"
	@echo "  make audit-unsafe     - Enforce // SAFETY: comments on unsafe blocks"
	@echo "  make ci               - All CI gates locally (fmt-check + lint + test + deny + audit-unsafe)"

# =============================================================================
# Format / Lint / Test
# =============================================================================

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all -- --check

.PHONY: lint
lint:
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: test
test:
	$(CARGO) test

.PHONY: spec
spec:
	quire validate --scope . 'spec/**/*.md' 'planning/**/*.md' 'plan/**/*.md'

.PHONY: msrv
msrv:
	$(CARGO) +1.75.0 test --locked

.PHONY: rustdoc
rustdoc:
	RUSTDOCFLAGS=-Dwarnings $(CARGO) doc --locked --no-deps

.PHONY: coverage
coverage:
	$(CARGO) llvm-cov --all-targets --locked --fail-under-lines 90

.PHONY: verify-evidence
verify-evidence:
	sha256sum --check evidence/manifest.sha256

.PHONY: build
build:
	$(CARGO) build --release

.PHONY: clean
clean:
	$(CARGO) clean

# =============================================================================
# Supply chain & safety
# =============================================================================

.PHONY: deny deny-advisories deny-bans deny-licenses deny-sources
deny: deny-advisories deny-bans deny-licenses deny-sources

deny-advisories:
	$(CARGO) deny check advisories

deny-bans:
	$(CARGO) deny check bans

deny-licenses:
	$(CARGO) deny check licenses

deny-sources:
	$(CARGO) deny check sources

.PHONY: cargo-audit
cargo-audit:
	$(CARGO) audit

.PHONY: audit-unsafe
audit-unsafe:
	bash scripts/check_unsafe_comments.sh

# =============================================================================
# Composite
# =============================================================================

.PHONY: ci
ci: fmt-check lint test deny audit-unsafe spec msrv rustdoc coverage verify-evidence
