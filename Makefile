# Docker management Makefile for Jito Tip Router

# Default target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "NCN Keeper commands:"
	@echo "  ncn-keeper-start         - Start the main keeper service"
	@echo "  ncn-keeper-start-metrics - Start the metrics-only keeper service" 
	@echo "  ncn-keeper-start-migrate - Start the migrate-only keeper service"
	@echo "  ncn-keeper-stop          - Stop and remove the main keeper service"
	@echo "  ncn-keeper-stop-metrics  - Stop and remove the metrics-only keeper service"
	@echo "  ncn-keeper-logs          - Follow logs for the main keeper service"
	@echo "  ncn-keeper-logs-metrics  - Follow logs for the metrics-only keeper service"
	@echo "  ncn-keeper-logs-migrate  - Follow logs for the migrate-only keeper service"
	@echo "  ncn-keeper-start-all     - Start all NCN keeper services"
	@echo "  operator-start           - Start the tip router operator service"
	@echo "  operator-stop            - Stop and remove the tip router operator service"
	@echo "  operator-logs            - Follow logs for the tip router operator service"
	@echo ""
	@echo "Utility commands:"
	@echo "  start-all                - Start all services (NCN keeper + operator)"
	@echo "  clean                    - Stop and remove all services"

# NCN Keeper services
.PHONY: ncn-keeper-start
ncn-keeper-start:
	docker compose --env-file cli/.env up -d --build jito-tip-router-ncn-keeper --remove-orphans

.PHONY: ncn-keeper-start-metrics
ncn-keeper-start-metrics:
	docker compose --env-file cli/.env up -d --build jito-tip-router-ncn-keeper-metrics-only --remove-orphans

.PHONY: ncn-keeper-start-migrate
ncn-keeper-start-migrate:
	docker compose --env-file cli/.env up -d --build jito-tip-router-ncn-keeper-migrate-only --remove-orphans

.PHONY: ncn-keeper-start-all
ncn-keeper-start-all: ncn-keeper-start ncn-keeper-start-metrics ncn-keeper-start-migrate
	@echo "All NCN keeper services started"

# Operator services
.PHONY: operator-start
operator-start:
	docker compose --env-file tip-router-operator-cli/.env up -d --build tip-router-operator-cli --remove-orphans

# NCN Keeper stop commands
.PHONY: ncn-keeper-stop
ncn-keeper-stop:
	docker stop jito-tip-router-ncn-keeper || true
	docker rm jito-tip-router-ncn-keeper || true

.PHONY: ncn-keeper-stop-metrics
ncn-keeper-stop-metrics:
	docker stop jito-tip-router-ncn-keeper-metrics-only || true
	docker rm jito-tip-router-ncn-keeper-metrics-only || true

# Operator stop commands
.PHONY: operator-stop
operator-stop:
	docker stop tip-router-operator-cli || true
	docker rm tip-router-operator-cli || true

# NCN Keeper logs
.PHONY: ncn-keeper-logs
ncn-keeper-logs:
	docker logs jito-tip-router-ncn-keeper -f

.PHONY: ncn-keeper-logs-metrics
ncn-keeper-logs-metrics:
	docker logs jito-tip-router-ncn-keeper-metrics-only -f

.PHONY: ncn-keeper-logs-migrate
ncn-keeper-logs-migrate:
	docker logs jito-tip-router-ncn-keeper-migrate-only -f

# Operator logs
.PHONY: operator-logs
operator-logs:
	docker logs tip-router-operator-cli -f
