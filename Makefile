.PHONY: build up down run start stop logs

# Build the Rust TUI binary
build:
	cargo build --release

# Build the Docker image
image:
	docker compose build

# Start the sandbox container (detached)
up:
	docker compose up -d

# Stop the sandbox container
down:
	docker compose down

# Start container + launch TUI
run: up build
	./target/release/azkaban

# Rebuild everything from scratch
rebuild: down
	docker compose build --no-cache
	cargo build --release

# View container logs (useful to see tool install progress)
logs:
	docker compose logs -f sandbox

# Check container status
status:
	@docker compose ps
