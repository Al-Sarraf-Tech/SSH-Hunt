SHELL := /bin/bash
COMPOSE := docker compose

.PHONY: ensure-env up down logs ps restart db-migrate db-seed test backup restore doctor firewall-open-24444

ensure-env:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "Created .env from .env.example"; \
	fi

up: ensure-env
	$(COMPOSE) up -d --build

down:
	$(COMPOSE) down

logs:
	$(COMPOSE) logs -f --tail=200

ps:
	$(COMPOSE) ps

restart:
	$(COMPOSE) restart

db-migrate:
	$(COMPOSE) run --rm --entrypoint /usr/local/bin/admin ssh-hunt migrate

db-seed:
	$(COMPOSE) run --rm --entrypoint /usr/local/bin/admin ssh-hunt seed

test:
	cd ssh-hunt && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace --all-features

backup:
	./scripts/backup.sh

restore:
	./scripts/restore.sh

firewall-open-24444:
	@if command -v firewall-cmd >/dev/null 2>&1; then \
		zones="$$(timeout 10 firewall-cmd --get-zones 2>/dev/null || echo public)"; \
		for z in $$zones; do \
			timeout 10 sudo firewall-cmd --zone $$z --add-port=24444/tcp || true; \
			timeout 10 sudo firewall-cmd --permanent --zone $$z --add-port=24444/tcp || true; \
		done; \
		timeout 10 sudo firewall-cmd --reload || true; \
	else \
		echo "firewall-cmd not found; skipping firewalld updates"; \
	fi

doctor:
	@echo "== Compose status =="
	@$(COMPOSE) ps
	@echo ""
	@echo "== Listener check (:24444) =="
	@ss -ltnp | grep ':24444' || true
	@echo ""
	@echo "== Public firewall ports (if firewalld available) =="
	@if command -v firewall-cmd >/dev/null 2>&1; then \
		timeout 10 firewall-cmd --zone=public --list-ports || echo "firewalld query timed out"; \
	else \
		echo "firewall-cmd not found"; \
	fi
