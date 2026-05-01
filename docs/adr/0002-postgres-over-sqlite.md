# ADR 0002: PostgreSQL for the persistence layer, not SQLite

- **Status**: Accepted
- **Date**: 2026-04-15

## Context

SSH-Hunt is an MMO-style honeypot — multiple processes (server,
admin CLI, future analytics jobs) and many concurrent player sessions
need consistent access to a shared world: players, missions, auctions,
chat history, leaderboards, and audit trails.

The persistence access pattern is mixed:

- **Hot read**: per-command profile and inventory lookups, ~1 per player
  command, expected single-digit milliseconds.
- **Burst write**: chat messages, auction bids, mission progress, mail.
- **Long-running query**: leaderboard snapshots, world events, admin
  stats — multi-row aggregations across all players.
- **Concurrent writers**: multiple game sessions actively mutating the
  same player rows (combat outcomes, wallet transactions).

The original prototype used SQLite. As soon as the second writer (admin
CLI alongside the server) showed up, lock contention surfaced; the
multi-writer story didn't scale. We considered staying on SQLite with
WAL mode, switching to PostgreSQL, or sharding by player into multiple
SQLite files.

## Decision

Use **PostgreSQL 16** as the single source of truth, accessed via
**sqlx 0.8** with the `runtime-tokio-rustls` and `postgres` features
enabled. Migrations live in `ssh-hunt/migrations/` and are applied via
the `admin migrate` subcommand at deploy time.

- True multi-writer concurrency with row-level locking — combat and
  auction code can update the same player without serializing through a
  global write lock.
- Server-side cursors and `LATERAL JOIN` for the leaderboard and
  history queries that don't fit cleanly in the SQLite relational model.
- Transactional DDL — migrations either apply atomically or roll back,
  which matters when the deploy adds a column referenced by the new
  release.
- Network-attached: server, admin, and any future analytics service
  (e.g. read-replica for dashboards) connect via `DATABASE_URL` without
  shared filesystem coupling.

## Alternatives considered

- **SQLite with WAL mode.** Solves single-writer-many-reader well, but
  the admin CLI is a real second writer and we expect more (analytics,
  scheduled jobs). WAL only allows one writer at a time across
  processes; bursts of concurrent writes would queue. The
  filesystem-coupling also blocks running the admin CLI from a
  different host.
- **Sharded SQLite per player or per zone.** Considered briefly. Ruled
  out because cross-shard queries (auctions, chat, leaderboard) would
  require fan-out at the application layer, reinventing what Postgres
  does natively. Operational complexity (many DB files, individual
  backup paths, schema-drift risk) outweighed the latency benefit.
- **Embedded key-value store (sled, redb).** Faster for the hot-read
  path, but loses SQL aggregation. The leaderboard and admin stats
  queries would each need a custom map/reduce — significant
  reimplementation cost for marginal latency gain.

## Consequences

- **Positive**: Multi-writer scales naturally; admin CLI and game server
  can both mutate state without coordination.
- **Positive**: Aggregation queries (leaderboard, stats, history)
  express in plain SQL.
- **Positive**: External `infra-postgres` deployment is supported via
  the `infra` external network in `docker-compose.yml` — same code path
  works for embedded and shared infrastructure.
- **Negative**: Postgres is a separate container with its own
  healthcheck path; cold-start ordering is `postgres → ssh-hunt`. See
  RUNBOOKS.md R100 for connection-refused recovery.
- **Negative**: sqlx compile-time query checking requires a live DB
  during `cargo build` for `query!` macros. Mitigated by using
  string-based queries (`sqlx::query("...").bind(...)`) instead of the
  macros, accepting runtime check in exchange for build-time
  decoupling.
- **Negative**: Migration coordination requires `admin migrate` before
  starting a new server version that adds columns. Documented in
  CLAUDE.md "Build & Test" section.
