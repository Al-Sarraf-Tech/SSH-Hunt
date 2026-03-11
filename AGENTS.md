# SSH-Hunt AGENTS

## What This Repo Does

`SSH-Hunt` is a Rust and Docker cyberpunk SSH game. Players connect over SSH, but gameplay is implemented against a simulated shell, virtual filesystem, scripting engine, and world state rather than the host OS.

## Main Entrypoints

- `docker-compose.yml`: runtime stack.
- `Makefile`: compose, migration, seed, test, doctor, runner, and firewall tasks.
- `ssh-hunt/Cargo.toml`: Rust workspace root.
- `ssh-hunt/crates/ssh_hunt_server/`: main server binary.
- `ssh-hunt/crates/shell/`, `world/`, `vfs/`, `scripts/`, `protocol/`: core game systems.
- `docs/` and `docs/examples/`: runtime config examples and ops docs.

## Commands

- `make ensure-env`
- `make up`
- `make db-migrate`
- `make db-seed`
- `make test`
- `make doctor`
- `docker compose config`

## Repo-Specific Constraints

- Player input must never reach real host command execution.
- Do not pass player-controlled strings to `std::process::Command` or shell invocations.
- Preserve container hardening and non-root runtime assumptions.
- Keep game command behavior in the `shell` crate and state mutations in the world layer.
- Do not weaken TLS or exposure assumptions around the public SSH service.

## Agent Notes

- Treat this repo as security-sensitive internet-facing software.
- Validate both compose configuration and the Rust test gate for touched server code.
