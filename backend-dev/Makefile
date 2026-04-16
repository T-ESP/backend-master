.PHONY: all down up reset

all: reset

down:
	docker compose down -v

up:
	docker compose up --build -d

reset: down up
	@echo "reset+run terminé"
