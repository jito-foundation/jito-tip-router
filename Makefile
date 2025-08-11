.PHONY: format format-coverage generate-client start start-metrics start-migrate stop stop-metrics stop-migrate logs logs-metrics logs-migrate help

.DEFAULT_GOAL := help

KEEPER_CONTAINER = jito-tip-router-ncn-keeper
METRICS_CONTAINER = jito-tip-router-ncn-keeper-metrics-only
MIGRATE_CONTAINER = jito-tip-router-ncn-keeper-migrate-only
ENV_FILE = cli/.env

define print_executing
	@echo "Executing: $(1)"
endef

format:
	$(call print_executing,cargo sort --workspace)
	cargo sort --workspace
	$(call print_executing,cargo fmt --all)
	cargo fmt --all
	$(call print_executing,cargo nextest run --all-features)
	cargo build-sbf --sbf-out-dir integration_tests/tests/fixtures
	SBF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run --all-features -E 'not test(ledger_utils::tests::test_get_bank_from_ledger_success)'
	$(call print_executing,cargo clippy --all-features)
	cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf
	$(call print_executing,cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b)
	cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b
	$(call print_executing,cargo-build-sbf)
	cargo-build-sbf
	@echo "Format and testing complete!"

format-coverage:
	$(call print_executing,cargo sort --workspace)
	cargo sort --workspace
	$(call print_executing,cargo fmt --all)
	cargo fmt --all
	$(call print_executing,cargo nextest run --all-features)
	cargo build-sbf --sbf-out-dir integration_tests/tests/fixtures
	SBF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run --all-features -E 'not test(ledger_utils::tests::test_get_bank_from_ledger_success)'
	$(call print_executing,cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info)
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info -- --skip "tip_router::bpf::set_merkle_root"
	$(call print_executing,cargo clippy --all-features)
	cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf
	$(call print_executing,cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b)
	cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b
	$(call print_executing,cargo-build-sbf)
	cargo-build-sbf
	@echo "Format, testing, and coverage complete!"

generate-client:
	@echo "Running full build process..."
	@echo "Step 1: Initial cargo build..."
	cargo b
	@echo "Step 2: Running shank-cli and generating clients..."
	./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients
	@echo "Step 3: Second cargo build..."
	cargo b
	@echo "Step 4: Building Solana programs..."
	cargo-build-sbf
	@echo "Step 5: Formatting code..."
	cargo fmt
	@echo "Client code generated successfully!!"

start:
	@echo "Building and starting keeper container..."
	docker compose --env-file $(ENV_FILE) up -d --build $(KEEPER_CONTAINER) --remove-orphans

start-metrics:
	@echo "Building and starting metrics container..."
	docker compose --env-file $(ENV_FILE) up -d --build $(METRICS_CONTAINER) --remove-orphans

start-migrate:
	@echo "Building and starting migrate container..."
	docker compose --env-file $(ENV_FILE) up -d --build $(MIGRATE_CONTAINER) --remove-orphans

stop:
	@echo "Stopping and removing keeper container..."
	docker stop $(KEEPER_CONTAINER) 2>/dev/null || echo "$(KEEPER_CONTAINER) not running"
	docker rm $(KEEPER_CONTAINER) 2>/dev/null || echo "$(KEEPER_CONTAINER) not found"

stop-metrics:
	@echo "Stopping and removing metrics container..."
	docker stop $(METRICS_CONTAINER) 2>/dev/null || echo "$(METRICS_CONTAINER) not running"
	docker rm $(METRICS_CONTAINER) 2>/dev/null || echo "$(METRICS_CONTAINER) not found"

stop-migrate:
	@echo "Stopping and removing migrate container..."
	docker stop $(MIGRATE_CONTAINER) 2>/dev/null || echo "$(MIGRATE_CONTAINER) not running"
	docker rm $(MIGRATE_CONTAINER) 2>/dev/null || echo "$(MIGRATE_CONTAINER) not found"

logs:
	@echo "Following logs for $(KEEPER_CONTAINER)..."
	docker logs $(KEEPER_CONTAINER) -f

logs-metrics:
	@echo "Following logs for $(METRICS_CONTAINER)..."
	docker logs $(METRICS_CONTAINER) -f

logs-migrate:
	@echo "Following logs for $(MIGRATE_CONTAINER)..."
	docker logs $(MIGRATE_CONTAINER) -f

# Default target
help:
help:
	@echo "Available targets:"
	@echo "  format          - Run full formatting, testing, and linting suite"
	@echo "  format-coverage - Same as format but with code coverage"
	@echo "  generate-client - Generate client code"
	@echo "  start           - Build and start keeper container"
	@echo "  start-metrics   - Build and start metrics container"
	@echo "  start-migrate   - Build and start migrate container"
	@echo "  stop            - Stop and remove keeper container"
	@echo "  stop-metrics    - Stop and remove metrics container"
	@echo "  stop-migrate    - Stop and remove migrate container"
	@echo "  logs            - Follow logs for jito-tip-router-ncn-keeper"
	@echo "  logs-metrics    - Follow logs for jito-tip-router-ncn-keeper-metrics-only"
	@echo "  logs-migrate    - Follow logs for jito-tip-router-ncn-keeper-migrate-only"
