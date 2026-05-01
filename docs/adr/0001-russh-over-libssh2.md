# ADR 0001: russh (pure Rust) for the SSH server, not libssh2

- **Status**: Accepted
- **Date**: 2026-04-15

## Context

SSH-Hunt is a public-facing honeypot listening on port 24444. The SSH
implementation is the entire attack surface for unauthenticated traffic
and must satisfy three constraints simultaneously:

1. **Memory safety under hostile input.** Every malformed packet, oversized
   payload, and protocol-violating handshake comes from an adversary. A
   single buffer-overflow CVE in the SSH layer compromises the whole
   container.
2. **Async/await native.** The server runs on tokio with thousands of
   concurrent player sessions. The SSH stack must integrate cleanly with
   `tokio::net::TcpListener`, not block worker threads on FFI calls.
3. **Tractable upgrade path.** SSH protocol vulnerabilities are routine
   (e.g. RUSTSEC-2026-0074, addressed by russh 0.59 in commit ee7a9fd).
   We need the audit and patch lifecycle to be a `cargo update` away,
   not a system-package coordination.

We considered three options: russh (pure Rust, async), libssh2 via the
`ssh2` crate (C bindings, sync), and shelling out to OpenSSH with
ProxyCommand-style adaptation.

## Decision

Use **russh** (currently 0.59) as the SSH server library.

- Pure Rust: no `unsafe` outside the dependency graph's transitive C
  shims (e.g. compression). Memory safety guarantees apply to the SSH
  packet parser itself.
- Tokio-native: `russh::server::Server` is implemented in async Rust;
  channel handling integrates directly with our session task model.
- Active maintenance: minor releases land within days of disclosed CVEs.

## Alternatives considered

- **libssh2 via the `ssh2` crate.** Synchronous C library wrapped in
  Rust bindings. Forces blocking IO inside `tokio::task::spawn_blocking`
  which inflates per-session thread cost from ~10 KiB (async task) to
  ~2 MiB (OS thread). C dependency means CVEs require system package
  updates, not `cargo update`. Audit history is also longer than russh's,
  but that's offset by the FFI-boundary risk on hostile input.
- **Shell out to OpenSSH.** Most battle-tested SSH implementation in
  existence. Ruled out because it would force us to stand up a full
  `sshd` per session and hijack `ForceCommand`/`AuthorizedKeysCommand`
  for game logic, fragmenting state and making the per-session game
  context awkward to thread through. Also breaks the goal of a single
  self-contained Rust binary.

## Consequences

- **Positive**: Single-binary deployment. CVE response is a `cargo
  update && cargo build --release && docker compose up -d`.
- **Positive**: Per-session memory cost is dominated by game state, not
  IO threading.
- **Positive**: Async cancellation works — when a player disconnects,
  the entire session task tree drops cleanly without orphaned threads.
- **Negative**: russh's user base is smaller than libssh2's; some
  obscure client quirks may require workarounds. Mitigated by the
  per-IP rate limiter and connection guards (see CLAUDE.md).
- **Negative**: russh major-version bumps occasionally break the
  `Server` trait API. Mitigated by pinning the exact version in
  workspace `Cargo.toml` and treating russh upgrades as their own
  commits with full test runs (e.g. ee7a9fd, 2a0edb5).
