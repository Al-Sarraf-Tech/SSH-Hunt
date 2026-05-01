# Architecture Decision Records

ADRs document load-bearing decisions in SSH-Hunt. Each one captures the
context, the decision, the alternatives, and the consequences — so future
readers (maintainers and any AI agent picking up work) can tell why the
code looks the way it does.

Format: `NNNN-short-kebab-title.md`. Status: Accepted | Superseded by
ADR-NNNN | Deprecated.

| # | Title | Status |
|---|---|---|
| [0001](0001-russh-over-libssh2.md) | russh (pure Rust) for the SSH server, not libssh2 | Accepted |
| [0002](0002-postgres-over-sqlite.md) | PostgreSQL for the persistence layer, not SQLite | Accepted |
| [0003](0003-story-mission-separation.md) | Separate story arcs from individual missions in the world model | Accepted |

When adding an ADR, copy the format from any existing entry and number it
sequentially. Do not renumber; reference older ADRs by number when
superseding.
