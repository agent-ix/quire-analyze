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
PYTHON ?= python3
QUIRE ?= quire

# The shared-assurance lane runs in its own interpreter. engineering-assurance
# declares jsonschema>=4.23,<5 and this repository's report validator is the
# Draft 7 `jsonschema` crate; both are right for their own job and neither may be
# bent to fit the other, so the Python half gets an environment of its own.
#
# This is not belt-and-braces. Under a jsonschema 3.2.0 interpreter the pinned
# mapping imports and appears to work, because the code paths that need a 4.x
# validator are exactly the ones a refusing record never reaches — which is the
# shape of a lane that looks green until it matters.
ASSURANCE_VENV ?= .venv-assurance
ASSURANCE_PYTHON ?= $(ASSURANCE_VENV)/bin/python

ASSURANCE_DIR := target/assurance
CENSUS_RESULT := $(ASSURANCE_DIR)/solver-state-census.json
ENGINES_RESULT := $(ASSURANCE_DIR)/engine-availability.json
PINS_RESULT := $(ASSURANCE_DIR)/shared-pins.json
COMPAT_RESULT := $(ASSURANCE_DIR)/legacy-compatibility.json
QUIRE_EXPORT := $(ASSURANCE_DIR)/quire-static-export.json
MSRV_RESULT := $(ASSURANCE_DIR)/msrv.jsonl
REVISION ?= $(shell git rev-parse HEAD)

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
	quire validate --scope . 'spec/**/*.md' 'planning/**/*.md' 'plan/**/*.md' 'reviews/**/*.md'

.PHONY: msrv
msrv:
	$(CARGO) +1.75.0 test --locked

.PHONY: rustdoc
rustdoc:
	RUSTDOCFLAGS=-Dwarnings $(CARGO) doc --locked --no-deps

.PHONY: coverage
coverage:
	$(CARGO) llvm-cov --all-targets --locked --fail-under-lines 90

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
# Shared assurance
# =============================================================================

$(ASSURANCE_PYTHON):
	$(PYTHON) -m venv $(ASSURANCE_VENV)
	$(ASSURANCE_VENV)/bin/pip install --quiet --disable-pip-version-check -r requirements-assurance.txt

.PHONY: assurance-env
assurance-env: $(ASSURANCE_PYTHON)

# The only target that runs a producer. Everything downstream consumes these
# files and refuses to create them: a driver that can produce its own inputs can
# produce a green run out of nothing.
.PHONY: assurance-inputs
assurance-inputs: assurance-env
	mkdir -p $(ASSURANCE_DIR)
	$(CARGO) run --quiet --example solver_state_census -- --json > $(CENSUS_RESULT)
	$(CARGO) run --quiet --example engine_availability -- --json > $(ENGINES_RESULT)
	$(ASSURANCE_PYTHON) scripts/check_shared_pins.py --json > $(PINS_RESULT)
	$(ASSURANCE_PYTHON) scripts/legacy_evidence_view.py --json > $(COMPAT_RESULT)
	$(QUIRE) coverage --scope . --json > $(QUIRE_EXPORT)
	rustup run 1.75.0 $(CARGO) check --locked --all-targets --message-format=json > $(MSRV_RESULT)

.PHONY: pins
pins: assurance-env
	$(ASSURANCE_PYTHON) scripts/check_shared_pins.py

.PHONY: compat-view
compat-view: assurance-env
	$(ASSURANCE_PYTHON) scripts/legacy_evidence_view.py
	$(ASSURANCE_PYTHON) scripts/legacy_evidence_view.py --mutation-probes

.PHONY: compat-fixtures
compat-fixtures: assurance-env
	$(ASSURANCE_PYTHON) scripts/legacy_evidence_view.py --write-fixtures

.PHONY: assurance-chain
assurance-chain: assurance-inputs
	$(PYTHON) scripts/assurance_chain.py --candidate-revision $(REVISION)

.PHONY: assurance
assurance: pins compat-view assurance-chain

# =============================================================================
# Composite
# =============================================================================

# The traced tests read the assurance gates' output, so the producers must
# already have run. They are a prerequisite rather than something a test creates
# for itself.
.PHONY: ci
ci: fmt-check lint assurance-inputs test deny audit-unsafe spec msrv rustdoc coverage assurance
