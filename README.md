# Azkaban

A Rust TUI for running agentic CLI tools inside a sandboxed Docker container, with built-in git worktree management and session tracking.

## Features

- **Sandboxed execution** — CLI tools run inside a Docker container so they can't damage your host system
- **Dynamic CLI tool support** — define any CLI tool in `config.toml` with install commands and flags
- **Project management** — auto-discover projects via glob patterns or list them explicitly
- **Git worktree support** — create, select, and delete worktrees per project
- **Session tracking** — view past sessions per worktree with summaries and timestamps
- **Fuzzy search** — type to filter projects instantly
- **Split-pane TUI** — projects, worktrees, and sessions shown side-by-side
- **Auto-install** — CLI tools are automatically installed inside the container on startup via entrypoint script
- **Docker Compose** — container lifecycle managed via `docker compose`, not the TUI

## Quick Start

```bash
# Build everything and launch
make run

# Or step by step:
cargo build --release          # Build the TUI
docker compose build           # Build the Docker image
docker compose up -d           # Start the sandbox container
./target/release/azkaban       # Launch the TUI
```

## Configuration

### config.toml

```toml
default_tool = "copilot"
project_dirs = ["~/projects/*"]
container_name = "azkaban-sandbox"

[[cli_tools]]
name = "copilot"
display_name = "GitHub Copilot"
install_cmd = "npm install -g @github/copilot"
cli_cmd = "copilot"
flags = ["--yolo"]

[[cli_tools]]
name = "claude"
display_name = "Claude Code"
install_cmd = "npm install -g @anthropic-ai/claude-code"
cli_cmd = "claude"
flags = ["--dangerously-skip-permissions"]

[[cli_tools]]
name = "gemini"
display_name = "Gemini CLI"
install_cmd = "npm install -g @google/gemini-cli"
cli_cmd = "gemini"
flags = ["--yolo"]
```

### docker-compose.yml

Volumes, environment variables, and platform settings are configured in `docker-compose.yml`. Edit it to:
- Mount additional project directories
- Add auth credentials (`~/.azure`, `~/.ssh`, etc.)
- Forward API keys via `.env` file

### .env

Store secrets in `.env` (gitignored):

```
ANTHROPIC_API_KEY=sk-...
GITHUB_TOKEN=ghp_...
```

## Keybindings

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate |
| `→` | Select / drill in |
| `←` | Go back |
| `Delete`/`Backspace` | Delete worktree (on worktree screen) |
| `Esc` | Quit (or clear search) |
| Type any letter | Fuzzy search projects |

## Makefile Commands

| Command | Description |
|---------|-------------|
| `make run` | Start container + launch TUI |
| `make build` | Build the Rust binary |
| `make image` | Build the Docker image |
| `make up` | Start the container |
| `make down` | Stop the container |
| `make rebuild` | Full rebuild from scratch |
| `make logs` | View container logs |
| `make status` | Check container status |

## Architecture

```
src/
├── main.rs       # Entry point, event loop, terminal setup
├── config.rs     # Config parsing (./config.toml)
├── worktree.rs   # Git worktree management
├── session.rs    # Session parsing + CLI tool command building
├── app.rs        # Application state machine
├── ui.rs         # TUI rendering (ratatui)
└── terminal.rs   # Terminal handoff for docker exec
```
