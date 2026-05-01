# SSH-Hunt Runbooks

Operator playbook for diagnosing and recovering from SSH-Hunt failures.
Each entry maps a symptom (log line, stderr message, or player report) to
probable cause, diagnostic steps, and recovery actions.

Logs are written as one JSON object per line at
`<LOG_DIR>/ssh-hunt-server.log.YYYY-MM-DD` when `LOG_DIR` is set, otherwise
to stderr. Filter with `jq`:

```bash
jq 'select(.level == "ERROR" or .level == "WARN")' \
  "$LOG_DIR/ssh-hunt-server.log.$(date -I)"
```

Container logs:

```bash
docker compose logs -f --tail=200 ssh-hunt
```

---

## R100 — PostgreSQL Connection Refused

**Symptom**

```
error: pool builder error: error connecting to server: Connection refused (os error 111)
```

```json
{"level":"ERROR","fields":{"message":"failed to open database pool","error":"...Connection refused..."}}
```

**Cause** Postgres container not started, healthcheck still failing, wrong
`DATABASE_URL`, or external `infra-postgres` network not attached.

**Diagnose**

```bash
docker compose ps postgres                         # local stack
docker network inspect infra                       # external infra setup
echo "$DATABASE_URL"                               # confirm URL host
psql "$DATABASE_URL" -c 'select 1'                 # direct probe
docker compose logs postgres --tail=80             # last events
```

**Recovery**

- Local stack: `docker compose up -d postgres` and wait for healthy state
  (~10s start_period, then 5s interval).
- External infra: `docker network connect infra ssh-hunt` if the container
  came up before the network existed.
- Stale pool: `docker compose restart ssh-hunt` to drop and re-establish
  connections.
- Wrong URL: edit `.env` `DATABASE_URL`, then `docker compose up -d
  --force-recreate ssh-hunt`.

---

## R101 — sqlx Migration Failure

**Symptom** `admin migrate` exits non-zero with `migration <N> failed` or
`column ... already exists`.

**Cause** Partial application after a prior crash, manual schema edit, or
migration file changed after deploy.

**Diagnose**

```bash
psql "$DATABASE_URL" -c "select * from _sqlx_migrations order by version;"
docker compose run --rm --entrypoint /usr/local/bin/admin ssh-hunt migrate
```

**Recovery**

- If a migration is half-applied, identify the failing version, undo the
  partial state by hand in `psql`, delete its row from `_sqlx_migrations`,
  then re-run `admin migrate`.
- Never edit a migration file already applied in production — write a new
  migration that fixes forward.

---

## R200 — russh Handshake Failure

**Symptom**

```
{"level":"WARN","fields":{"message":"ssh accept error","error":"...protocol error..."}}
```

Player sees `Connection closed by remote host` immediately after
`ssh -p 24444 user@host`.

**Cause** Host key missing or unreadable, key algorithm rejected by
the player's client, or russh version mismatch with a peer (rare after
russh 0.59 bump).

**Diagnose**

```bash
ls -l /data/secrets/ssh_host_ed25519
docker compose exec ssh-hunt ls -l /data/secrets/ssh_host_ed25519
ssh -vvv -p 24444 player@host 2>&1 | grep -iE 'kex|cipher|host key'
```

**Recovery**

- Missing key: server auto-generates an ephemeral key and logs a warning
  with `error = ?err`. Fix by mounting a persistent key path so player
  trust survives restarts.
- Permission: file must be readable by the server UID. `chmod 600` and
  matching ownership inside the container.
- Algorithm refusal: confirm client supports `ssh-ed25519`. Older clients
  may require enabling it explicitly.

---

## R201 — Per-IP Connection Rate Limit Tripped

**Symptom**

```json
{"level":"WARN","fields":{"message":"connection rejected: rate limit","ip":"...","window_secs":10}}
```

Player sees rapid drops on subsequent reconnect attempts.

**Cause** `connection_rate_max` (default 5 in `connection_rate_window_secs`,
default 10s) exceeded by a single IP. Working as designed for honeypot
abuse cases; legitimate when a player reconnects in a tight loop after a
crashed terminal.

**Recovery**

- Wait for the window to clear (default 10s).
- For long-lived NAT'd networks (school, conference), raise
  `server.connection_rate_max` and `server.max_connections_per_ip` in
  `config.yaml`, then `docker compose up -d --force-recreate ssh-hunt`.

---

## R300 — Mission State Corrupt

**Symptom** Player reports a mission stuck in `Active` after objective
completion, or `progress` field above 100, or mission visible in `missions
list` but absent from `MissionDefinition` table.

```json
{"level":"ERROR","fields":{"message":"mission state inconsistent","mission":"...","player":"..."}}
```

**Cause** Race in mission update path (rare), seed data drift between
deploys, or manual SQL edit.

**Diagnose**

```bash
psql "$DATABASE_URL" -c "select code, state, progress, updated_at from player_missions where player_id=(select id from players where username='<u>');"
psql "$DATABASE_URL" -c "select code from missions order by code;"
```

**Recovery**

- Single mission stuck: `update player_missions set state='Available',
  progress=0 where player_id=... and code='...';` then ask player to
  re-trigger the start command. Document the recovery in the player's
  audit row.
- Drift between code and DB: `admin seed` is idempotent and re-asserts
  shop/lore rows. Mission definitions live in
  `crates/world/src/missions.rs` and `lib.rs` MissionDefinition data; a
  redeploy recovers them.
- Bulk corruption: stop the server, take a `pg_dump`, then run
  `admin migrate` to re-apply the latest schema and rerun seeds.

---

## R400 — NPC AI Loop Hang

**Symptom** Combat or chat turn never completes; all sessions interacting
with the NPC stall on the next request. Server still accepts new SSH
connections.

```json
{"level":"WARN","fields":{"message":"npc tick exceeded budget","npc":"...","ms":...}}
```

**Cause** Rhai script entered an unbounded loop or recursion.
`ScriptPolicy` enforces `consumed_ops` and elapsed-ms ceilings; if a
script bypasses the engine entirely (e.g. blocking IO inside a host fn),
the timeout fires too late.

**Diagnose**

```bash
# Sample the server thread to see who's stuck.
docker compose exec ssh-hunt sh -c 'kill -QUIT 1' && \
  docker compose logs ssh-hunt --tail=200 | grep -A30 backtrace
docker compose exec ssh-hunt ls -lh /data/scripts
```

**Recovery**

- Kill the stuck connection only: identify by `session_id` in logs; the
  server's per-session task isolation contains the impact.
- Restart the server: `docker compose restart ssh-hunt` — drops every
  active player session, last-resort.
- Patch the script: tighten `ScriptPolicy` `max_ops` / `max_runtime_ms`
  in the affected NPC profile, then redeploy.

---

## R500 — World Reset Failure

**Symptom** `admin` reset command exits non-zero, or world appears
half-reset (some auctions cleared, others linger; player wallets not
zeroed).

**Cause** Reset is a multi-statement transaction; partial failure leaves
mixed state if an exception fires mid-transaction without rollback.

**Diagnose**

```bash
psql "$DATABASE_URL" -c "select count(*) from auctions where closed_at is null;"
psql "$DATABASE_URL" -c "select count(*) from player_missions where state='Active';"
psql "$DATABASE_URL" -c "select pid, state, query_start, query from pg_stat_activity where state != 'idle';"
```

**Recovery**

- If the reset transaction is still running, wait — long resets on busy
  worlds can take minutes. Look for `pg_stat_activity` rows that haven't
  moved.
- If a connection is wedged, terminate it: `select pg_terminate_backend(<pid>);`
  then re-run the reset.
- For half-reset state, the safe path is `pg_dump` the current world,
  then re-run reset; never leave a world with mixed pre/post state in
  production.

---

## R600 — Player Session Timeout

**Symptom** Player drops mid-session with no error, or sees
`Connection to <host> closed by remote host.` after 10-30 minutes idle.

```json
{"level":"INFO","fields":{"message":"session closed","reason":"idle_timeout","session":"..."}}
```

**Cause** TCP keepalive missed, intermediate NAT timed out the connection,
or russh enforced its own idle limit. The honeypot intentionally keeps
sessions short for some scenarios; long idle drops are usually network.

**Diagnose**

```bash
# Server-side: count drops over the last hour by reason.
jq 'select(.fields.message=="session closed") | .fields.reason' \
  "$LOG_DIR/ssh-hunt-server.log.$(date -I)" | sort | uniq -c
# Client-side: ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 ...
```

**Recovery**

- Player can add `ServerAliveInterval 30` to their `~/.ssh/config` for
  the SSH-Hunt host.
- For NAT-ed players (corporate networks), `ServerAliveInterval 15` is
  more aggressive; documented in `docs/GAMEPLAY.md`.
- Server-side, no action — drops are expected. If a specific session
  closed unexpectedly, pull its `session_id` from logs and audit the
  preceding 50 lines for actual errors.
