# ADR 0003: Separate story arcs from individual missions in the world model

- **Status**: Accepted
- **Date**: 2026-04-15

## Context

SSH-Hunt has two related but distinct content concepts:

- **Missions**: discrete, completable tasks with explicit objectives and
  rewards (e.g. `nav-101`, `signal-trace`). Each mission has a code,
  state machine (Locked → Available → Active → Completed), progress
  counter, and suggested command. Missions are short-lived from the
  player's perspective.
- **Story arcs / campaign chapters**: narrative threads that span
  multiple missions and trigger NPC mail, world events, and lore drops
  (e.g. the Ghost Rail conspiracy, the Black Ice Storm). A story
  doesn't have a "complete" state in the same way — it has chapters
  that unlock based on mission completion combinations and player
  choices.

The original design lumped everything into one `MissionDefinition`
table with optional fields for narrative content. This created two
problems:

1. **Mission queries became slow.** Listing available missions for a
   player joined against narrative columns the UI never reads in the
   list view; story-only rows polluted the mission list with rows that
   had no objective.
2. **Story progression became hard to reason about.** Triggering a
   chapter transition required scanning every mission row and applying
   a chapter-specific rule, and adding a new chapter meant editing
   mission code paths.

## Decision

Split the world model into two parallel concerns:

- `MissionDefinition` (in `crates/world/src/missions.rs` plus the
  partial split in `lib.rs`): pure mission data — code, title,
  objective, reward, prerequisites. State per player lives in
  `player_missions` table.
- `Campaign` and `Story` constructs (in `lib.rs`, planned to migrate
  to `campaign.rs` per the S-tier overhaul design): chapter
  definitions, mail trigger dispatch, narrative-state hooks. State per
  player lives in `player_story_progress` table.

Mission completion fires a "campaign tick" that the story system
consumes; the story system never reads mission data directly. Stories
read their own progress table and decide what to unlock or trigger
next.

## Alternatives considered

- **Keep one model with optional narrative fields.** Continues the
  original problem: queries slow down, schema gets cluttered, and the
  "is this row a mission or a story node?" check leaks across every
  call site.
- **One model with discriminator column** (`type = 'mission' | 'story'`).
  Cleaner than the optional-field approach but still couples the two
  state machines. Story progression can't easily reference a set of
  missions completed in any order, only sequential chains.
- **Story as data, missions as code.** Considered embedding story rules
  directly in Rust source. Ruled out because content writers (and
  future tooling) need to iterate on story arcs without recompiling.

## Consequences

- **Positive**: Mission list query is a single-table scan with a fixed
  column set; UI rendering is fast and predictable.
- **Positive**: Adding a new story arc means editing one file
  (campaign-table source) and adding chapter rows. No mission code
  paths change.
- **Positive**: Story state survives mission re-runs (e.g. when a
  mission is reset for QA, story progression continues independently).
- **Negative**: Two tables to keep consistent on player creation and
  reset. Mitigated by writing both updates inside a single transaction
  in the player CRUD path.
- **Negative**: A "completion" event has two listeners (mission
  state transition and story tick). If the story tick panics, the
  mission row is still marked complete. Acceptable because the story
  tick is best-effort (mail/lore can be redelivered) but documented in
  RUNBOOKS.md R300 (mission state corrupt) for the rare reverse case.
