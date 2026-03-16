# CLAUDE.md — SSH-Hunt

Cyberpunk SSH game and terminal learning MMO. Fully simulated shell and world written
in Rust. Publicly playable at `ssh -p 24444 <username>@ssh-hunt.appnest.cc`.

Deployed via Docker Compose. The Rust workspace lives inside the `ssh-hunt/` directory.

---

## Organizational Directive (Claude Only)

> **This directive applies ONLY when Claude Code is in use — it is a standing operational policy, not a suggestion.**
>
> Claude operates in this repository as a structured internal engineering organization: single point of contact, adaptive team complexity (Tier 0–4), mandatory review on all work, batch processing, and parallelization where safe. Full directive: `~/.claude/CLAUDE.md`.

---

## Architecture

```
ssh-hunt/                 # Rust workspace root
  crates/
    ssh_hunt_server/      # Main server binary (SSH listener, session handler)
    shell/                # Simulated shell engine
    vfs/                  # Virtual filesystem
    world/                # Game world, NPCs, economy
    ui/                   # TUI/output rendering
    protocol/             # SSH protocol handling
    admin/                # Admin tools (migrate, seed)
    scripts/              # Lua/scripting engine
docker/                   # Docker context files
volumes/                  # Named volume mount points
docs/                     # Architecture and ops docs
migrations/               # SQLite/DB migration files
```

---

## Common Commands

```bash
# Start the stack
docker compose up -d --build   # or: make up

# View logs
docker compose logs -f --tail=200   # or: make logs

# Check status
docker compose ps              # or: make ps

# Restart all services
docker compose restart         # or: make restart

# Run DB migrations
make db-migrate

# Seed test data
make db-seed

# Firewall: open port 24444
make firewall-open-24444

# Stop stack
docker compose down            # or: make down

# CI runner (self-hosted GitHub Actions)
docker compose -f docker-compose.runner.yml up -d
```

---

## Environment Setup

```bash
# First run: create .env from template
make ensure-env
# Then edit .env with your secrets (DB URL, JWT secret, Cloudflare tunnel token, etc.)
```

Never commit `.env`. The `docker-compose.yml` uses `${VAR:?error}` guards.

---

## Rust Build (inside Docker)

The Rust workspace is built inside the container image via the `Dockerfile`.
Do not run `cargo build` directly on the host for production artifacts — use
`docker compose up --build`.

For local development iteration:
```bash
# Enter the builder stage manually if needed
docker compose run --rm ssh-hunt cargo test --workspace
```

---

## CI/CD Pipeline

Five workflows: `ci.yml`, `security.yml`, `codeql.yml`, `deep-security-sweep.yml`,
`docker.yml`. All must pass before merge to `main`.

---

## Security Constraints

- The game server is designed for hostile internet exposure.
- All player input is handled inside the simulated shell — it must never reach
  the host OS. Do not add code that passes player-supplied strings to `std::process::Command`
  or any shell invocation.
- Do not weaken authentication or session isolation.
- `ssh-hunt` binary must never run as root.
- Cloudflare Tunnel handles TLS termination. Do not add direct TLS to the game port.

---

## Coding Conventions

- Rust 2021 edition. Zero clippy warnings (`-D warnings`).
- `anyhow::Result` for error propagation. No `.unwrap()` in non-test code.
- All game state mutations go through the world engine — no direct DB writes
  from shell handlers.
- New game commands go in `shell/` and require both a unit test and a
  regression test entry.

---

## Validation

```bash
make doctor                    # environment and config health check
docker compose config          # validate compose syntax
cargo clippy --workspace --all-targets -- -D warnings   # (inside container)
cargo test --workspace                                   # (inside container)
```

---

## Toolchain

| Tool | Path | Version |
|---|---|---|
| rustc | `/usr/bin/rustc` | 1.93.1 (Fedora dnf) |
| cargo | `/usr/bin/cargo` | 1.93.1 (Fedora dnf) |
| rustfmt | `/usr/bin/rustfmt` | 1.93.1 |
| rust-analyzer | `/usr/bin/rust-analyzer` | 1.93.1 |

Rust is system-installed via dnf, not rustup shims. `rustup` is present at `~/.rustup` but
its shims are not active — `/usr/bin/rustc` takes priority.
`~/.cargo/bin/` is in PATH for user-installed cargo tools (cargo-audit, cargo-deny, aihelp, cyberdeck).


---

## CI/CD Pipeline (Enforced)

This repository's CI/CD pipeline is **generated and managed by the Haskell CI Orchestrator** (`~/git/haskell-ci-orchestrator`). Do not manually edit `.github/workflows/ci.yml` — changes will be overwritten on the next sync.

**Directives:**
- All CI/CD runs through the unified `ci.yml` pipeline (lint → test → security → sbom → docker → integration → release)
- **Never release for macOS** — no macOS runners, no macOS release targets
- **Never use the Gentoo runner** — all jobs target `[self-hosted, unified-all]`
- **Never touch `haskell-money` or `haskell-ref`** — hard-denied by the orchestrator
- Pipeline changes go through the orchestrator catalog (`CI.Catalog`), not direct YAML edits
- The orchestrator validates, generates, and syncs workflows across all 15 repos
