# Make will use bash instead of sh
SHELL := /usr/bin/env bash

.PHONY: help
help:
	@echo ' Run Services:'
	@echo '    make run   		Run the default binary.'
	@echo ''
	@echo ' Development:'
	@echo '    make build   	Build the code base incrementally (fast) for dev.'
	@echo '    make rebuild   	Sync dependencies and builds the code base from scratch (slow).'
	@echo '    make release   	Build & test binaries and then build & publish container images (slow).'
	@echo '    make container      Build the container images.'
	@echo '    make fix   		Fix linting issues as reported by clippy.'
	@echo '    make format   	Format call code according to cargo fmt style.'
	@echo '    make test   	Test all crates.'

# "---------------------------------------------------------"
# Run targets
# "---------------------------------------------------------"
.PHONY: run
run:
	@source scripts/run_default.sh


# "---------------------------------------------------------"
# Development make targets
# "---------------------------------------------------------"

.PHONY: build
build:
	@source scripts/build.sh


.PHONY: rebuild
rebuild:
	@source scripts/rebuild.sh


.PHONY: release
release:
	@source scripts/release.sh


.PHONY: container
container:
	@source scripts/container.sh


.PHONY: doc
doc:
	@source scripts/doc.sh


.PHONY: fix
fix:
	@source scripts/fix.sh


.PHONY: format
format:
	@source scripts/format.sh


.PHONY: test
test:
	@source scripts/test.sh


.PHONY: sbe
sbe:
	@source scripts/sbe.sh
