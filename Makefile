.PHONY: all down up reset test

all: reset

down:
	docker compose down -v

up:
	docker compose up --build -d

reset: down up
	@echo "reset+run terminé"

# Run the ai-service Python test-suite inside its container (mock-based,
# no DB/LLM/network needed).
test:
	docker compose run --rm --no-deps ai-service python -m pytest tests/ -v
