# SSH-Hunt

[![CI](https://github.com/Al-Sarraf-Tech/SSH-Hunt/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Al-Sarraf-Tech/SSH-Hunt/actions/workflows/ci.yml)
[![Release](https://img.shields.io/badge/release-v1.0.2-blue)](https://github.com/Al-Sarraf-Tech/SSH-Hunt/releases/tag/v1.0.2)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-green)](#license)

### **[The Ghost Rail Conspiracy — Full Story & Lore](STORY.md)**

> CI runs on self-hosted runners governed by the [Haskell Orchestrator](https://github.com/Al-Sarraf-Tech/Haskell-Orchestrator). Lint, test, security, SBOM, Docker, and release jobs are unified in a single pipeline.

SSH-Hunt is a publicly playable cyberpunk SSH game and terminal-learning MMO. Connect with any SSH client, learn real shell commands through story-driven missions, hack NPCs, and unravel two conspiracies — the Ghost Rail Conspiracy and the Crystal Array expansion — in a living world where defeated characters are replaced by harder successors.

**What it is:**
- A Rust SSH server that presents a fully simulated shell and virtual filesystem — zero host access
- 104 missions across 7 difficulty tiers with a branching conspiracy narrative
- 19 named NPCs with dossiers, mail, combat profiles, and succession mechanics
- 12-chapter campaign mode spanning two story arcs, narrated by EVA, the adaptive training AI
- NPC hacking with a hybrid duel + shell-challenge bonus system
- PvP/PvE combat, auction house, scripts market, and daily rewards
- Training Sim, NetCity multiplayer hub, and REDLINE timed mode

**Play now — no account required:**

```
ssh -p 24444 <username>@ssh-hunt.appnest.cc
```

---

## Table of Contents

- [The World](#the-world)
- [Quick Start](#quick-start)
- [How to Play](#how-to-play)
- [Mission System](#mission-system)
- [NPC System](#npc-system)
- [Campaign Mode](#campaign-mode)
- [Combat System](#combat-system)
- [Command Reference](#command-reference)
- [Architecture](#architecture)
- [Security](#security)
- [Deployment and Ops](#deployment-and-ops)
- [CI/CD Pipeline](#cicd-pipeline)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)

---

## The World

Ghost Rail — NetCity's transit backbone — went dark three nights ago. CorpSim says it was a power failure. The logs say otherwise. A beacon called GLASS-AXON-13 keeps repeating. Vault-sat-9 is offline. And a name that shouldn't exist appeared in the auth log: **wren**.

You are a recruit in CorpSim's "training simulation." What they don't tell you is that every file in the sim is pulled from live infrastructure. You're not practicing — you're investigating.

The conspiracy unfolds across 12 campaign chapters, 104 missions, and interactions with 19 NPCs — from allies feeding you intel to executives trying to bury the evidence. NPCs can be hacked, defeated, and replaced. The world adapts.

The story spans two arcs: the Ghost Rail Conspiracy (Chapters 1–7) and the Crystal Array expansion (Chapters 8–12), where players uncover Project ZENITH — a mass surveillance AI.

**[Full story with spoilers: STORY.md](STORY.md)**

---

## Quick Start

### Play on the public server

```bash
ssh -p 24444 <username>@ssh-hunt.appnest.cc
```

No registration. Pick any username on first connect and EVA will walk you through setup.

### Self-host

```bash
git clone https://github.com/Al-Sarraf-Tech/SSH-Hunt.git
cd SSH-Hunt
./scripts/install.sh          # create volume dirs and default config
cp .env.example .env          # edit .env — set POSTGRES_PASSWORD and secrets before proceeding
make up
```

Verify:

```bash
make ps
make logs
make doctor
```

Connect locally:

```bash
ssh -p 24444 <username>@localhost
```

---

## How to Play

### First login

```text
tutorial start          # EVA guides you through 6 shell basics
campaign start          # begin the Ghost Rail investigation
missions                # see the full mission board
accept keys-vault       # required mission — register your SSH key
```

### Progression flow

```
Tutorial (nav-101 → pipe-101) ──> Keys Vault ──> Starter Missions
    ──> NetCity Unlock ──> Intermediate ──> Advanced ──> Expert
    ──> Ghost Rail Campaign (Ch 1–7) ──> Boss Fight (Wren)
    ──> Crystal Array Campaign (Ch 8–12) ──> Supreme Boss (APEX)
```

### Key systems

| System | Commands | Description |
|--------|----------|-------------|
| Tutorial | `tutorial start/next/reset` | 6-step interactive walkthrough with EVA |
| Campaign | `campaign start/next` | 12-chapter story across two arcs |
| Missions | `missions`, `accept`, `submit` | 104 missions across 7 tiers |
| NPCs | `dossier`, `mail`, `hack` | 19 characters with combat and dialogue |
| Combat | `hack <npc>`, `pvp challenge` | Hybrid duel + shell challenge system |
| Economy | `shop`, `auction`, `daily` | Currency, items, and daily rewards |
| Social | `chat`, `mail`, `leaderboard` | Global chat, NPC mail, rankings |

### Register SSH key and unlock NetCity

```bash
# Local machine:
ssh-keygen -t ed25519 -a 64 -f ~/.ssh/ssh-hunt_ed25519

# In-game:
keyvault register       # paste your public key
submit keys-vault
mode netcity            # after completing one starter mission
```

---

## Mission System

104 missions organized into 7 tiers. Each teaches progressively harder shell skills while advancing the narrative:

| Tier | Missions | Rep | Shell Skills | Story Layer |
|------|----------|-----|-------------|-------------|
| Tutorial | 5 | 5 | pwd, ls, cat, echo, grep, pipes | EVA onboarding |
| Starter | 14 | 10 | grep, sort, uniq, wc, find, redirect | Surface anomalies — first clues |
| Intermediate | 15 | 15 | head/tail, awk, cut, tee, xargs, tr | The insider thread — evidence |
| Advanced | 29 | 20 | awk, sed, diff, regex, multi-file pipelines | The conspiracy — full picture |
| Expert | 12 | 30 | ROT13, multi-source grep, full pipelines | Endgame — prosecution and reckoning |
| Gateway | 1 | 15 | SSH key management | Keys Vault (required unlock) |
| Legendary | 28 | 50 | base64, hex, multi-file correlation | Crystal Array — ZENITH and APEX |

Each mission has a story beat, hint, suggested command, and server-side validation against the virtual filesystem.

---

## NPC System

19 named NPCs with full backstories, dossier profiles, and an in-game mail system that triggers on mission completion.

```text
dossier                 # list all discovered NPCs
dossier KES             # full profile for Kestrel
mail inbox              # check your NPC mail
mail read 1             # read message #1
```

NPCs unlock progressively — a character's dossier appears only after you complete the mission that reveals them.

**EVA** is your constant companion — an adaptive training AI embedded in the simulation. She narrates the tutorial, provides campaign chapter briefings, and delivers context-aware hints via the `eva` command.

---

## Campaign Mode

12 chapters across two story arcs:

| Ch | Title | Theme |
|----|-------|-------|
| 1 | The Blackout | Tutorial + orientation |
| 2 | Surface Anomalies | First clues + NPC introductions |
| 3 | The Insider Thread | Evidence gathering |
| 4 | The Conspiracy | Revelation |
| 5 | Confrontation | NPC hacking unlocks |
| 6 | The Reckoning | Endgame evidence chain |
| 7 | The Reply | Boss fight + sequel hook |
| 8 | Crystal Array | Enter the sector, discover ZENITH surveillance AI |
| 9 | The Mirror | Find Obsidian's ZENITH clone, meet Quicksilver & Volt |
| 10 | The Defector | Cipher's knowledge, ZENITH's scope revealed |
| 11 | Ghost Protocol | Confront Spectre, Wren's true motive |
| 12 | APEX | Final boss — shut down ZENITH, battle APEX |

```text
campaign start          # begin chapter 1
campaign                # show current objectives
campaign next           # advance after completing an objective
eva                     # context-aware guidance from EVA
eva hint                # hint for your active mission
eva lore                # background for the current chapter
```

---

## Combat System

### PvP/PvE stance

```text
stance                  # show current stance
stance pvp              # other players can challenge you
stance pve              # safe from player challenges (default)
```

### NPC hacking

Requires NetCity mode. NPCs have combat stats scaled by story importance (40 HP easy → 280 HP supreme boss):

```text
hack FER                # start hack against Ferro
hack attack             # deal 14–30 damage
hack defend             # halve next incoming damage
hack script quickhack   # script-based attack
hack solve              # solve the shell challenge for bonus damage
```

Before each attack, you can run the NPC's shell challenge against the VFS and use `hack solve` to verify — correct solutions deal bonus damage on the next hit.

### Crystal Array combat NPCs

| Callsign | NPC | Difficulty | HP | Challenge |
|----------|-----|-----------|-----|-----------|
| `VLT` | Volt | Hard | 140 | Map the power grid |
| `QSV` | Quicksilver | Very Hard | 160 | Crack the network topology |
| `CPH` | Cipher | Very Hard | 160 | Break ZENITH's encryption |
| `SPC` | Spectre | Extreme | 180 | Face the assassin |
| `ZEN` | Zenith | Extreme | 200 | Confront the surveillance AI |
| `OBS` | Obsidian | Boss | 220 | Sever The Reach |
| `APX` | APEX | Supreme Boss | 280 | Kill the god |

### NPC succession

When an NPC is defeated:
1. Recorded in the **NetCity History Ledger** (`history` command)
2. A successor takes the role with harder stats: `HP + (total_defeats × 5)`, capped at 300
3. Boss NPCs (Wren, Zenith, Obsidian, APEX) are not replaceable — they return for every player

Crystal Array successor pools:

| Role | Succession Chain |
|------|-----------------|
| Power Engineer | Volt → Amp → Ohm → Watt → Tesla → Farad |
| Network Architect | Quicksilver → Mercury → Platinum → Gallium → Iridium → Osmium |
| Cryptanalyst | Cipher → Enigma → Vigenere → Playfair → Atbash → Vernam |
| Ghost Operative | Spectre → Phantom → Wraith → Shade → Ghost → Revenant |

```text
history                 # view the NetCity history ledger
```

---

## Command Reference

### Core
`help` `guide [quick|full|shell]` `tutorial [start|next|reset|1-6]` `missions` `accept <code>` `submit <code>` `briefing [code]` `mode <training|netcity|redline>` `gate` `status` `events` `leaderboard [N]` `daily` `tier <noob|gud|hardcore>` `settings flash <on|off>`

### Intel
`dossier [callsign]` `mail [inbox|read N|count]` `eva [hint|status|lore]` `campaign [start|next]` `history`

### Combat
`stance [pvp|pve]` `hack <callsign>` `hack attack|defend|script <name>|solve` `pvp roster` `pvp challenge <username>` `pvp attack|defend|script <name>`

### Economy
`inventory` `shop list|buy <sku>` `auction list|sell|bid|buyout` `scripts market` `scripts run <name>`

### Social
`chat <global|sector|party> <message>` `keyvault register`

### Shell (simulated — no host access)
`pwd` `cd` `ls [-l] [-la]` `cat [-n]` `head [-n N]` `tail [-n N]` `grep [-i] [-v] [-n] [-c] [-E]` `find [-name] [-type]` `wc [-l] [-w] [-c]` `sort [-r] [-n] [-u] [-k N] [-t]` `uniq [-c] [-d]` `cut [-f] [-d] [-c]` `sed` `awk [-F]` `tr` `tee` `xargs [-I{}]` `echo [-n] [-e]` `printf` `seq` `nl` `column [-t]` `paste` `base64` `cp [-r]` `mv` `rm [-r]` `mkdir` `touch` `diff` `env` `export`

---

## Architecture

SSH-Hunt is a Rust workspace (`edition = "2021"`) with 8 crates:

```
ssh-hunt/crates/
├── ssh_hunt_server    # Main binary — russh SSH daemon, command dispatch, VFS bootstrap
├── shell              # Tokenizer + pipeline executor (|, &&, ||, >, >>)
├── vfs                # In-memory virtual filesystem (no host FS access)
├── world              # Missions, players, NPCs, combat, economy, campaign state
├── scripts            # Sandboxed Rhai scripting engine for player scripts
├── ui                 # Terminal rendering: banners, themes, progress meters, ANSI/ASCII fallback
├── protocol           # Shared types: Mode, MissionStatus, MailMessage, combat state, etc.
└── admin              # Admin CLI binary (migrate, seed, backup)

ssh-hunt/tests/        # Integration + regression test suite
```

### How the server works

Players connect over standard SSH (port 24444 externally, 22222 inside the container). The game server is a custom [`russh`](https://github.com/warp-tech/russh)-based SSH daemon. All gameplay executes against:

| Layer | What it is | Host access? |
|-------|-----------|-------------|
| Virtual filesystem (VFS) | In-memory tree of game files | None |
| Simulated world state | Missions, NPCs, economy, duels — all in-process | None |
| Sandboxed script engine | Rhai scripts players write and run | None |
| PostgreSQL | Persistent player data, mission progress, leaderboard | Via `DATABASE_URL` only |

**Runtime container:** Built from `debian:bookworm-slim`, runs as UID 10001 (non-root), read-only root filesystem, all capabilities dropped, `no-new-privileges`, 768 MB memory cap, 1.5 CPU limit, separate `ingress` and `backend` Docker networks.

### Test suite

138+ tests across the workspace:

| Suite | Location | Count |
|-------|----------|-------|
| Regression (integration) | `tests/tests/regression.rs` | 55 |
| Shell + VFS unit tests | `crates/shell`, `crates/vfs` | 12 |
| Game command unit tests | `crates/ssh_hunt_server` | 37 |
| UI unit tests | `crates/ui` | 34 |

---

## Security

SSH-Hunt is designed for hostile internet exposure.

### Hard guarantees (enforced at compile time)

- `#![forbid(unsafe_code)]` on all gameplay crates
- No `std::process::Command` or `tokio::process::Command` in game server paths
- No host filesystem access outside mounted runtime data volumes
- No host service control or shell breakout path exists

### Runtime hardening

- **Per-IP rate limiting:** Custom TCP accept loop with `ConnectionTracker` enforces per-IP concurrent connection limits and a sliding-window connection rate limit. Excess connections are rejected before the SSH handshake — hostile scanners consume no server resources beyond the TCP close.
- **Rhai script injection fix:** The grep Rhai handler escapes the needle (`\` and `"`) before constructing the Rhai expression, preventing attacker-controlled mission input from injecting arbitrary script.
- **Combat state hardening:** All NPC duel state lookups use `ok_or_else()` instead of `unwrap()`, eliminating panic paths reachable from hostile combat commands.
- **Line buffer cap:** Input line buffer is capped at 4 KiB per connection to prevent memory exhaustion from clients that never send a newline.
- **Breakout detection:** Probing attempts trigger immediate permanent score-zero and disconnect.

### Container security

| Control | Value |
|---------|-------|
| User | UID 10001 (non-root) |
| Root FS | `read_only: true` |
| Capabilities | All dropped |
| New privileges | `no-new-privileges: true` |
| PID limit | 256 |
| Memory limit | 768 MB |
| CPU limit | 1.5 cores |
| Tmpfs | `/tmp` (64 MB, noexec) + `/run` (16 MB, noexec) |

Full security documentation: [`docs/SECURITY.md`](docs/SECURITY.md)

---

## Deployment and Ops

### Make targets

```bash
make up                     # start all services
make down                   # stop all services
make ps                     # container status
make logs                   # tail logs
make restart                # restart services
make doctor                 # health check
make firewall-open-24444    # open SSH port in firewalld
make firewall-status        # show firewall state
make db-migrate             # run database migrations
make db-seed                # seed initial data
make test                   # run full test suite
make backup                 # backup player data
make restore                # restore from backup
```

### Networking

| Interface | Address |
|-----------|---------|
| Host SSH port | `0.0.0.0:24444` (configurable) |
| Container listen | `0.0.0.0:22222` |
| Cloudflare Tunnel target | `ssh://localhost:24444` |
| Docker internal | `ssh://ssh-hunt:22222` |

### Self-hosted runners

```bash
cp .env.runner.example .env.runner
make runner-up              # start self-hosted CI runners
make runner-logs            # tail runner output
make runner-ps              # runner container status
```

Full deployment documentation: [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md)
Runner setup: [`docs/SELF_HOSTED_RUNNER.md`](docs/SELF_HOSTED_RUNNER.md)

---

## CI/CD Pipeline

This project is governed by the [Haskell Orchestrator](https://github.com/Al-Sarraf-Tech/Haskell-Orchestrator) — a multi-agent CI/CD governance framework that handles pre-push validation, code quality enforcement, and release management across the Al-Sarraf-Tech organization.

All CI runs on **self-hosted runners**. The unified pipeline (`.github/workflows/ci.yml`):

| Job | What it does | Trigger |
|-----|-------------|---------|
| **Governance** | `repo-guard` verifies repository ownership before any jobs run | Every push/PR |
| **Lint** | `cargo fmt --check` + `cargo clippy -D warnings` | Every push/PR |
| **Test** | `cargo test --workspace` | Every push/PR |
| **Security** | gitleaks, cargo audit, cargo deny, trivy, CodeQL, osv-scanner | Every push/PR |
| **SBOM** | Syft SPDX + CycloneDX generation | `main` branch |
| **Docker** | Build + push game server image | `main` + tags |
| **Release** | SHA256 checksums, GitHub Release assets | Tags only |

Pipeline features:
- Concurrency groups cancel in-progress runs on the same ref
- Cargo registry/git/target caching for fast rebuilds
- Weekly scheduled run (Monday 04:00 UTC) for dependency freshness
- Pre-push orchestrator scan: `orchestrator-enterprise scan .github/workflows/`

---

## Configuration

Environment template: `.env.example`

| Variable | Default | Description |
|----------|---------|-------------|
| `SSH_HUNT_PORT` | `24444` | Host-published SSH port |
| `SSH_HUNT_LISTEN` | `0.0.0.0:22222` | Container listen address |
| `DATABASE_URL` | `postgres://ssh_hunt:...@postgres:5432/ssh_hunt` | PostgreSQL connection |
| `POSTGRES_PASSWORD` | *(required)* | Database password — set before first run |
| `GAME_CONFIG_PATH` | `/data/config.yaml` | Server config (rate limits, UI defaults, REDLINE timer) |
| `HIDDEN_OPS_PATH` | `/data/secrets/hidden_ops.yaml` | Secret missions config |
| `SSH_HOST_KEY_PATH` | `/data/secrets/ssh_host_ed25519` | Persistent host key |
| `ADMIN_SECRET_PATH` | `/data/secrets/admin.yaml` | Admin credentials |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

Runtime secrets (never committed): `admin.yaml`, `hidden_ops.yaml`, `ssh_host_ed25519`

The host key persists across restarts via the `./volumes/ssh-hunt/secrets/` bind mount. Players will only see a host key warning on the very first deployment.

---

## Troubleshooting

**Service not reachable:**
```bash
make ps
ss -ltnp | grep 24444
make doctor
make firewall-open-24444
```

**Connection refused:** Containers stopped, missing `.env`, or NAT/port-forward not configured.

**NetCity locked:** Complete `keys-vault` + at least one starter mission, then reconnect using the registered SSH key (`ssh -i ~/.ssh/ssh-hunt_ed25519 ...`).

**Host key warning on reconnect:** Only expected on first deploy. If it recurs, the `/data/secrets/ssh_host_ed25519` volume was lost — regenerate and warn players.

**Cross-terminal compatibility:** Server normalizes CRLF, handles CR/LF/CRLF Enter variants, ignores escape sequences, and auto-falls back to ASCII frames on narrow terminals.

---

## Contributing

- Read [`docs/GAMEPLAY.md`](docs/GAMEPLAY.md), [`docs/SECURITY.md`](docs/SECURITY.md), [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md), and [`docs/SELF_HOSTED_RUNNER.md`](docs/SELF_HOSTED_RUNNER.md)
- Follow [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Run `make test` before opening a PR

CI gate (must pass):
```bash
cd ssh-hunt
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

---

## License

Dual-licensed under:
- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
