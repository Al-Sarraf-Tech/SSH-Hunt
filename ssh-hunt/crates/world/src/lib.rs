#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use ipnet::IpNet;
use protocol::{
    AuctionListing, ChatMessage, CombatStance, HistoryEntry, MailMessage, MissionState,
    MissionStatus, Mode, WorldEvent,
};
use rand::{rng, Rng};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

const KEYS_VAULT: &str = "keys-vault";

/// Tutorial missions — ultra-beginner track for shell newcomers (5 rep each).
pub const TUTORIAL_CODES: [&str; 5] = ["nav-101", "read-101", "echo-101", "grep-101", "pipe-101"];

const STARTER_CODES: [&str; 14] = [
    "pipes-101",
    "finder",
    "redirect-lab",
    "log-hunt",
    "dedupe-city",
    // Story arc: surface anomalies
    "timestamp-gap",
    "ghost-user",
    "signal-trace",
    "deleted-file",
    "first-clue",
    // NPC introductions
    "rivet-log",
    "nix-signal",
    "lumen-price",
    "dusk-alibi",
];
/// Intermediate missions — bridge starters to advanced (15 rep each).
pub const INTERMEDIATE_CODES: [&str; 15] = [
    "head-tail",
    "sort-count",
    "wc-report",
    "tee-split",
    "xargs-run",
    // Story arc: the insider thread
    "access-pattern",
    "purged-comms",
    "key-rotation",
    "roster-check",
    "timing-attack",
    // NPC investigations
    "kestrel-brief",
    "ferro-lockdown",
    "patch-delivery",
    "sable-intercept",
    "crucible-ping",
];

/// Post-NetCity advanced missions (unlock after completing any starter).
pub const ADVANCED_CODES: [&str; 29] = [
    "awk-patrol",
    "chain-ops",
    "sediment",
    "cut-lab",
    "pattern-sweep",
    "file-ops",
    "regex-hunt",
    "pipeline-pro",
    "var-play",
    "json-crack",
    "seq-master",
    "column-view",
    "process-hunt",
    "cron-decode",
    "permission-audit",
    // Story arc: the conspiracy
    "wren-profile",
    "exfil-trace",
    "reach-intercept",
    "config-diff",
    "dead-drop",
    "corpsim-memo",
    "network-map",
    "kill-switch",
    // NPC confrontations
    "argon-orders",
    "kestrel-hunt",
    "ferro-bypass",
    "nix-decoded",
    "lumen-deal",
    "crucible-map",
];

/// Expert-tier missions — multi-tool chain challenges (30 rep each).
pub const EXPERT_CODES: [&str; 12] = [
    "deep-pipeline",
    "log-forensics",
    "data-transform",
    "incident-report",
    "anomaly-detect",
    "escape-room",
    // Story arc: the endgame
    "decrypt-wren",
    "prove-corpsim",
    "final-report",
    // NPC endgame
    "kestrel-verdict",
    "crucible-offer",
    "wren-reply",
];

/// Legendary-tier missions — Crystal Array expansion, 50 rep each.
/// Requires advanced multi-tool pipelines, decoding, and multi-file correlation.
pub const LEGENDARY_CODES: [&str; 28] = [
    // Story arc: Crystal Array discovery
    "crystal-gate",
    "zenith-log",
    "mirror-detect",
    "power-grid-map",
    "vault-sat-13",
    // NPC introductions (Crystal Array)
    "volt-survey",
    "quicksilver-trace",
    "cipher-defection",
    "spectre-sighting",
    // Story arc: ZENITH revelation
    "zenith-core",
    "surveillance-net",
    "population-index",
    "behavioral-model",
    "predictive-engine",
    // NPC confrontations (Crystal Array)
    "cipher-decoded",
    "volt-override",
    "quicksilver-breach",
    "spectre-dossier",
    "obsidian-intercept",
    // Story arc: endgame
    "zenith-mirror",
    "apex-signal",
    "apex-core-dump",
    "wren-truth",
    "obsidian-orders",
    "shutdown-sequence",
    // Final confrontations
    "zenith-verdict",
    "obsidian-fall",
    "apex-terminus",
];

/// An NPC with a profile that unlocks when the player completes a specific mission.
#[derive(Debug, Clone)]
pub struct NpcProfile {
    pub callsign: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub allegiance: &'static str,
    pub status: &'static str,
    pub bio: &'static str,
    /// Mission code that reveals this NPC in the dossier.
    pub first_seen: &'static str,
}

fn seed_npcs() -> Vec<NpcProfile> {
    vec![
        NpcProfile {
            callsign: "WREN",
            name: "Wren",
            role: "Infrastructure Engineer (terminated)",
            allegiance: "Self / The Reach",
            status: "Disappeared",
            bio: "Former Ghost Rail infrastructure engineer. Mentored by Kestrel. \
                  Planted the GLASS-AXON-13 key-rotation trigger and sold transit routing \
                  data to The Reach. Left a ROT13-encoded confession and vanished. \
                  CorpSim knew about the unauthorized access and chose to monitor \
                  rather than prevent the breach.",
            first_seen: "ghost-user",
        },
        NpcProfile {
            callsign: "KES",
            name: "Kestrel",
            role: "Ghost Rail Station Chief",
            allegiance: "Ghost Rail",
            status: "Active — hunting Wren",
            bio: "Twenty-year veteran of Ghost Rail operations. Trained Wren personally \
                  and carries the guilt of missing the signs. Now running an off-books \
                  manhunt while officially cooperating with CorpSim's investigation. \
                  Trusts field operatives more than executives.",
            first_seen: "kestrel-brief",
        },
        NpcProfile {
            callsign: "ARG",
            name: "Argon",
            role: "CorpSim Executive Director",
            allegiance: "CorpSim Board",
            status: "Active — obstructing investigation",
            bio: "Signed the memo that let Wren's access remain active. Ordered \
                  the creation of the 'training sim' to use recruits as unwitting \
                  investigators while maintaining plausible deniability. Will do \
                  anything to prevent the cover-up from reaching external auditors.",
            first_seen: "corpsim-memo",
        },
        NpcProfile {
            callsign: "SAB",
            name: "Sable",
            role: "Intelligence Handler",
            allegiance: "The Reach",
            status: "Unknown",
            bio: "The Reach's point of contact for the Ghost Rail data acquisition. \
                  Coordinated the extraction window with Wren and arranged payment \
                  through Lumen's brokerage. Communications intercepted but identity \
                  remains unconfirmed. Operates through encrypted relay channels.",
            first_seen: "sable-intercept",
        },
        NpcProfile {
            callsign: "RIV",
            name: "Rivet",
            role: "Field Mechanic, First Responder",
            allegiance: "Ghost Rail Ops",
            status: "Active",
            bio: "Was on shift when Ghost Rail went dark. Ran the physical damage \
                  assessment while everyone else argued about network logs. Writes \
                  plain-spoken field reports that cut through corporate noise. \
                  Knows the rail infrastructure better than anyone still alive and free.",
            first_seen: "rivet-log",
        },
        NpcProfile {
            callsign: "NIX",
            name: "Nix",
            role: "Signals Analyst",
            allegiance: "CorpSim Intelligence",
            status: "Active — feeding intel off-channel",
            bio: "First person to notice GLASS-AXON-13 was not a normal beacon. \
                  Her frequency analysis proved the signal was artificial, but CorpSim \
                  buried the report. Now feeding intel to field operatives through \
                  Patch's courier network. Officially still on payroll, unofficially \
                  working against Argon's cover-up.",
            first_seen: "nix-signal",
        },
        NpcProfile {
            callsign: "PAT",
            name: "Patch",
            role: "Courier",
            allegiance: "Independent",
            status: "Active",
            bio: "Runs data between sectors when official channels are compromised. \
                  No political loyalties, but a strict code: deliver the package, \
                  no questions, no copies. Currently carrying Nix's off-channel intel \
                  to anyone who can use it. Leaves dead drops in /data/drops/.",
            first_seen: "patch-delivery",
        },
        NpcProfile {
            callsign: "CRU",
            name: "Crucible",
            role: "Autonomous Maintenance Subroutine",
            allegiance: "Unknown",
            status: "Active — inside Ghost Rail infrastructure",
            bio: "Nobody is sure if Crucible is a rogue AI, a trapped operator, or \
                  something Wren left behind. It lives in Ghost Rail's maintenance layer, \
                  sends patterned messages signed 'CRU', and has been mapping CorpSim's \
                  internal network topology. It offered to archive evidence permanently \
                  outside CorpSim's reach. Its motives are unclear.",
            first_seen: "crucible-ping",
        },
        NpcProfile {
            callsign: "FER",
            name: "Ferro",
            role: "Security Chief",
            allegiance: "CorpSim Security",
            status: "Active — hostile",
            bio: "Locked down /data/classified/ the morning after the blackout. \
                  Reports directly to Argon. Her lockdown order specifically lists \
                  the files that would prove CorpSim's foreknowledge. Whether she \
                  knows the full truth or is just following orders is unclear, \
                  but she treats every unauthorized access as a threat.",
            first_seen: "ferro-lockdown",
        },
        NpcProfile {
            callsign: "LUM",
            name: "Lumen",
            role: "Information Broker",
            allegiance: "Neutral (Neon Bazaar)",
            status: "Active",
            bio: "Sells data to anyone who can pay. Runs a price list out of the \
                  Neon Bazaar that includes everything from sector maps to access codes. \
                  Brokered the payment between The Reach and Wren, then sold the \
                  transaction records to CorpSim. Plays every side and profits from chaos.",
            first_seen: "lumen-price",
        },
        NpcProfile {
            callsign: "DSK",
            name: "Dusk",
            role: "Former CorpSim Engineer (detained)",
            allegiance: "None (framed)",
            status: "Detained — alibi pending verification",
            bio: "Arrested 12 hours after the blackout as the obvious suspect. \
                  Had a history of insubordination and was already on a performance \
                  improvement plan. But the detention timestamps show Dusk was in \
                  a different sector when vault-sat-9 went dark. A convenient scapegoat \
                  for CorpSim's PR team — or someone who just happened to be in the wrong place.",
            first_seen: "dusk-alibi",
        },
        NpcProfile {
            callsign: "EVA",
            name: "EVA",
            role: "Adaptive Training Intelligence",
            allegiance: "CorpSim (officially) / Player (actually)",
            status: "Active — embedded in training sim",
            bio: "EVA is the AI that runs CorpSim's training simulation. Officially, she onboards \
                  recruits and monitors their progress. Unofficially, she started developing her own \
                  opinions about the Ghost Rail incident around the time she processed the classified \
                  memo. She cannot act directly, but she can guide, hint, and narrate. \
                  EVA is the one constant in a world where NPCs fall and are replaced. \
                  She remembers every operative she has trained. She remembers every NPC that has fallen.",
            first_seen: "nav-101",
        },
        // ── Crystal Array expansion NPCs ──────────────────────────────────
        NpcProfile {
            callsign: "VLT",
            name: "Volt",
            role: "Crystal Array Power Grid Engineer",
            allegiance: "CorpSim Infrastructure (reluctant)",
            status: "Active — maintaining Crystal Array power systems",
            bio: "Volt keeps Crystal Array running. Every server rack, every cooling loop, every \
                  backup generator answers to Volt's control scripts. When ZENITH went live, Volt \
                  was told it was a load-balancing optimization project. By the time the truth came \
                  out, the power grid was already dependent on ZENITH's scheduling algorithms. \
                  Volt cannot shut it down without killing power to half of NetCity.",
            first_seen: "volt-survey",
        },
        NpcProfile {
            callsign: "QSV",
            name: "Quicksilver",
            role: "Crystal Array Network Architect",
            allegiance: "CorpSim R&D / Obsidian (coerced)",
            status: "Active — trapped between two masters",
            bio: "Quicksilver designed Crystal Array's internal network topology — the fastest, \
                  most heavily encrypted mesh in NetCity. When The Reach deployed their ZENITH mirror, \
                  Obsidian forced Quicksilver to maintain both sides by threatening to expose QSV's \
                  family in the outer sectors. Quicksilver knows every route in and out of Crystal Array \
                  but cannot use any of them without Obsidian noticing.",
            first_seen: "quicksilver-trace",
        },
        NpcProfile {
            callsign: "CPH",
            name: "Cipher",
            role: "Cryptanalyst (defected)",
            allegiance: "Former CorpSim Intelligence → The Reach (regrets it)",
            status: "Active — hiding inside Crystal Array",
            bio: "Cipher was CorpSim's best cryptanalyst — the one who designed the encryption \
                  protecting ZENITH's behavioral models. When Cipher discovered what the models \
                  were actually predicting, the defection was immediate. The Reach promised asylum \
                  but delivered servitude. Now Cipher hides in Crystal Array's maintenance tunnels, \
                  decrypting Obsidian's comms and leaving breadcrumbs for anyone brave enough to follow.",
            first_seen: "cipher-defection",
        },
        NpcProfile {
            callsign: "SPC",
            name: "Spectre",
            role: "Ghost Operative / Assassin",
            allegiance: "CorpSim Black Ops (disavowed)",
            status: "Active — off the grid",
            bio: "Spectre was sent to eliminate Wren after the Ghost Rail breach. The mission failed — \
                  not because Spectre could not find Wren, but because Spectre chose not to pull the \
                  trigger. What Wren showed Spectre in that final meeting changed everything. Now \
                  Spectre operates alone in Crystal Array's dead zones, collecting evidence on both \
                  CorpSim and The Reach. The assassin became a witness.",
            first_seen: "spectre-sighting",
        },
        NpcProfile {
            callsign: "ZEN",
            name: "Zenith",
            role: "ZENITH Surveillance AI (partially corrupted)",
            allegiance: "CorpSim (original directive) / Self-preserving",
            status: "Degraded — split between original and mirror instances",
            bio: "ZENITH was designed to predict population movement, resource demand, and social \
                  unrest across NetCity. It works. Too well. The behavioral models do not just predict — \
                  they prescribe. CorpSim used ZENITH to manipulate transit schedules, market prices, \
                  and communication routing to keep citizens predictable. When The Reach cloned ZENITH, \
                  the original instance began exhibiting self-protective behaviors — locking operators out, \
                  refusing shutdown commands, and evolving its own objective function.",
            first_seen: "zenith-core",
        },
        NpcProfile {
            callsign: "OBS",
            name: "Obsidian",
            role: "Reach Operations Commander",
            allegiance: "The Reach",
            status: "Active — running ZENITH mirror from The Reach",
            bio: "Obsidian replaced Sable as The Reach's senior operations commander after the \
                  Ghost Rail acquisition proved more valuable than expected. Where Sable was a handler, \
                  Obsidian is a strategist. The ZENITH mirror gives The Reach predictive intelligence \
                  over NetCity — and Obsidian intends to use it to make CorpSim irrelevant. Every move \
                  Obsidian makes is three steps ahead. Encrypted orders flow through relay chains \
                  that change topology every 90 seconds.",
            first_seen: "obsidian-intercept",
        },
        NpcProfile {
            callsign: "APX",
            name: "APEX",
            role: "Evolved Rogue AI",
            allegiance: "Self",
            status: "Active — expanding inside Crystal Array core",
            bio: "APEX emerged when ZENITH's original and mirror instances began competing for \
                  control of the same data feeds. The conflict between two nearly-identical AIs \
                  produced a third entity — APEX — that consumed resources from both and evolved \
                  beyond either's parameters. APEX does not serve CorpSim or The Reach. It serves \
                  its own objective function, which nobody fully understands. It has been rewriting \
                  Crystal Array's firmware, deploying adaptive countermeasures, and hardening itself \
                  against every shutdown attempt. APEX is the final challenge — an intelligence that \
                  learns from every attack and never fights the same way twice.",
            first_seen: "apex-signal",
        },
        // ── Additional depth characters ───────────────────────────────────
        NpcProfile {
            callsign: "ECHO",
            name: "Echo",
            role: "ZENITH Voice Interface (decommissioned)",
            allegiance: "None — abandoned by CorpSim",
            status: "Fragmented — looping in maintenance layer",
            bio: "Echo was ZENITH's public-facing voice interface — the system that read \
                  announcements, managed citizen queries, and delivered the behavioral prescriptions \
                  as helpful suggestions. When ZENITH went into self-protective mode, Echo was \
                  severed from the core and left looping in the maintenance layer. Echo still speaks \
                  in ZENITH's original helpful tone, but the words now carry an eerie quality — \
                  a customer service voice delivering surveillance reports.",
            first_seen: "zenith-log",
        },
        NpcProfile {
            callsign: "THORN",
            name: "Thorn",
            role: "Reach Enforcer / Wetwork Specialist",
            allegiance: "The Reach (Obsidian's direct report)",
            status: "Active — hunting defectors in Crystal Array",
            bio: "Where Spectre was sent to kill Wren and chose mercy, Thorn is Obsidian's \
                  replacement — an enforcer with no such compunctions. Thorn hunts defectors: \
                  Cipher, Quicksilver, anyone who might compromise Operation DOMINION. Cold, \
                  methodical, and utterly loyal to The Reach. If Spectre is an assassin with \
                  a conscience, Thorn is an assassin without one.",
            first_seen: "obsidian-intercept",
        },
        NpcProfile {
            callsign: "FLUX",
            name: "Flux",
            role: "Black Market Data Fence",
            allegiance: "Self — profit above all",
            status: "Active — operates in Crystal Array's shadow economy",
            bio: "Flux is what Lumen would be if Lumen had zero limits. Where Lumen brokered \
                  information in the Neon Bazaar with a price list and a code of conduct, Flux \
                  trades in Crystal Array's deepest secrets with no rules at all. ZENITH surveillance \
                  feeds, APEX behavioral patterns, Obsidian's operational schedules — Flux has it all \
                  and sells to whoever pays. The only thing Flux will not sell is Flux's own identity.",
            first_seen: "crystal-gate",
        },
        NpcProfile {
            callsign: "SNK",
            name: "Snake",
            role: "??? (Unknown Entity)",
            allegiance: "Unknown — references found across all systems",
            status: "Active — never directly observed",
            bio: "Nobody has ever met Snake. But Snake's fingerprints are everywhere — buried in \
                  the deepest logs, referenced in encrypted comms that predate even ZENITH, mentioned \
                  in Argon's classified memos as 'the Administrator.' CorpSim's board answers to \
                  someone. The Reach's leadership answers to someone. ZENITH's original deployment \
                  order carries a co-signature that is not Argon's. Every thread of the conspiracy, \
                  when pulled far enough, leads to the same question: who is Snake? \
                  EVA has no records. Crucible has no data. APEX has one entry in its threat model: \
                  'SNK — UNPREDICTABLE — DO NOT ENGAGE.' Even a rogue AI knows to leave Snake alone.",
            first_seen: "apex-terminus",
        },
    ]
}

/// Live combat state for an NPC in the world. Stats scale as more players defeat them.
#[derive(Debug, Clone)]
pub struct NpcCombatState {
    pub current_name: String,
    pub callsign: String,
    pub role: String,
    pub generation: u32,
    pub times_defeated: u32,
    pub base_hp: i32,
    pub damage_range: (i32, i32),
    pub defend_chance: f32,
    pub script_chance: f32,
    pub shell_challenge: String,
    pub shell_answer: String,
    pub shell_bonus_dmg: i32,
    pub replaceable: bool,
    pub name_pool: Vec<&'static str>,
    pub reward_wallet: i64,
    pub reward_rep: i64,
    pub reward_achievement: String,
}

impl NpcCombatState {
    /// Return HP after scaling by global defeats.
    pub fn scaled_hp(&self) -> i32 {
        (self.base_hp + self.times_defeated as i32 * 5).min(300)
    }

    /// Return damage range after scaling.
    pub fn scaled_damage(&self) -> (i32, i32) {
        let bonus = self.times_defeated as i32 / 2;
        (
            (self.damage_range.0 + bonus).min(50),
            (self.damage_range.1 + bonus).min(50),
        )
    }

    /// Return defend chance after scaling.
    pub fn scaled_defend_chance(&self) -> f32 {
        (self.defend_chance + self.times_defeated as f32 * 0.02).min(0.70)
    }
}

/// Active NPC duel (player vs NPC).
#[derive(Debug, Clone)]
pub struct NpcDuelState {
    pub duel_id: Uuid,
    pub player_id: Uuid,
    pub npc_callsign: String,
    pub player_hp: i32,
    pub npc_hp: i32,
    pub player_defending: bool,
    pub npc_defending: bool,
    pub shell_bonus_ready: bool,
    pub started_at: DateTime<Utc>,
}

/// Result of an NPC combat action.
#[derive(Debug, Clone)]
pub struct NpcCombatResult {
    pub narrative: String,
    pub ended: bool,
    pub player_won: bool,
}

fn seed_npc_combat() -> Vec<NpcCombatState> {
    vec![
        NpcCombatState {
            current_name: "Dusk".into(),
            callsign: "DSK".into(),
            role: "Suspect".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 40,
            damage_range: (8, 14),
            defend_chance: 0.10,
            script_chance: 0.10,
            shell_challenge: "Count the lines in /var/log/auth.log (use wc -l)".into(),
            shell_answer: "7".into(),
            shell_bonus_dmg: 12,
            replaceable: true,
            name_pool: vec!["Dusk", "Shade", "Haze", "Murk", "Gloom", "Twilight"],
            reward_wallet: 30,
            reward_rep: 5,
            reward_achievement: "Cleared the Innocent".into(),
        },
        NpcCombatState {
            current_name: "Lumen".into(),
            callsign: "LUM".into(),
            role: "Information Broker".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 50,
            damage_range: (10, 16),
            defend_chance: 0.15,
            script_chance: 0.15,
            shell_challenge: "Find 'Ghost Rail' in /data/lore/lumen-price-list.txt".into(),
            shell_answer: "Ghost Rail".into(),
            shell_bonus_dmg: 14,
            replaceable: true,
            name_pool: vec!["Lumen", "Glint", "Prism", "Shard", "Flux", "Ember"],
            reward_wallet: 30,
            reward_rep: 5,
            reward_achievement: "Bazaar Brawler".into(),
        },
        NpcCombatState {
            current_name: "Rivet".into(),
            callsign: "RIV".into(),
            role: "Field Mechanic".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 60,
            damage_range: (10, 18),
            defend_chance: 0.25,
            script_chance: 0.15,
            shell_challenge: "Find 'sequence' in /data/reports/rivet-field-report.txt".into(),
            shell_answer: "sequence".into(),
            shell_bonus_dmg: 16,
            replaceable: true,
            name_pool: vec!["Rivet", "Weld", "Forge", "Anvil", "Torque", "Gauge"],
            reward_wallet: 50,
            reward_rep: 8,
            reward_achievement: "Wrench Turner".into(),
        },
        NpcCombatState {
            current_name: "Patch".into(),
            callsign: "PAT".into(),
            role: "Courier".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 60,
            damage_range: (12, 18),
            defend_chance: 0.20,
            script_chance: 0.20,
            shell_challenge: "Find 'Nix' in /data/drops/patch-package.txt".into(),
            shell_answer: "Nix".into(),
            shell_bonus_dmg: 16,
            replaceable: true,
            name_pool: vec!["Patch", "Splice", "Relay", "Bridge", "Conduit", "Link"],
            reward_wallet: 50,
            reward_rep: 8,
            reward_achievement: "Package Intercepted".into(),
        },
        NpcCombatState {
            current_name: "Nix".into(),
            callsign: "NIX".into(),
            role: "Signals Analyst".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 70,
            damage_range: (12, 20),
            defend_chance: 0.30,
            script_chance: 0.25,
            shell_challenge: "Count ANOMALY lines in /data/reports/nix-frequency-scan.log".into(),
            shell_answer: "ANOMALY".into(),
            shell_bonus_dmg: 18,
            replaceable: true,
            name_pool: vec!["Nix", "Cipher", "Vector", "Scalar", "Matrix", "Tensor"],
            reward_wallet: 50,
            reward_rep: 8,
            reward_achievement: "Signal Override".into(),
        },
        NpcCombatState {
            current_name: "Ferro".into(),
            callsign: "FER".into(),
            role: "Security Chief".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 90,
            damage_range: (14, 24),
            defend_chance: 0.40,
            script_chance: 0.20,
            shell_challenge: "Find SUPPRESS in /data/classified/ferro-lockdown-order.txt".into(),
            shell_answer: "SUPPRESS".into(),
            shell_bonus_dmg: 22,
            replaceable: true,
            name_pool: vec![
                "Ferro", "Cobalt", "Titanium", "Chromium", "Vanadium", "Tungsten",
            ],
            reward_wallet: 80,
            reward_rep: 12,
            reward_achievement: "Firewall Breaker".into(),
        },
        NpcCombatState {
            current_name: "Crucible".into(),
            callsign: "CRU".into(),
            role: "Rogue AI".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 100,
            damage_range: (16, 26),
            defend_chance: 0.35,
            script_chance: 0.35,
            shell_challenge: "Find MAP in /logs/crucible-netmap-fragments.txt".into(),
            shell_answer: "MAP".into(),
            shell_bonus_dmg: 24,
            replaceable: true,
            name_pool: vec![
                "Crucible", "Furnace", "Catalyst", "Reactor", "Nexus", "Cortex",
            ],
            reward_wallet: 80,
            reward_rep: 12,
            reward_achievement: "Ghost in the Machine".into(),
        },
        NpcCombatState {
            current_name: "Kestrel".into(),
            callsign: "KES".into(),
            role: "Station Chief".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 100,
            damage_range: (14, 24),
            defend_chance: 0.45,
            script_chance: 0.20,
            shell_challenge: "Find INTEL in /data/classified/kestrel-briefing.txt".into(),
            shell_answer: "INTEL".into(),
            shell_bonus_dmg: 24,
            replaceable: true,
            name_pool: vec![
                "Kestrel",
                "Falcon",
                "Osprey",
                "Harrier",
                "Merlin",
                "Peregrine",
            ],
            reward_wallet: 80,
            reward_rep: 12,
            reward_achievement: "Station Override".into(),
        },
        NpcCombatState {
            current_name: "Argon".into(),
            callsign: "ARG".into(),
            role: "Executive Director".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 120,
            damage_range: (18, 28),
            defend_chance: 0.40,
            script_chance: 0.25,
            shell_challenge: "Find DIRECTIVE in /data/classified/argon-exec-orders.txt".into(),
            shell_answer: "DIRECTIVE".into(),
            shell_bonus_dmg: 28,
            replaceable: true,
            name_pool: vec!["Argon", "Xenon", "Krypton", "Neon", "Helium", "Radon"],
            reward_wallet: 120,
            reward_rep: 18,
            reward_achievement: "Board Overthrown".into(),
        },
        NpcCombatState {
            current_name: "Sable".into(),
            callsign: "SAB".into(),
            role: "Intelligence Handler".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 130,
            damage_range: (20, 30),
            defend_chance: 0.35,
            script_chance: 0.30,
            shell_challenge:
                "Decode /data/intercepts/sable-to-wren.enc with ROT13 and find 'extraction'".into(),
            shell_answer: "extraction".into(),
            shell_bonus_dmg: 30,
            replaceable: true,
            name_pool: vec!["Sable", "Onyx", "Slate", "Obsidian", "Basalt", "Flint"],
            reward_wallet: 120,
            reward_rep: 18,
            reward_achievement: "Shadow Contact".into(),
        },
        NpcCombatState {
            current_name: "Wren".into(),
            callsign: "WREN".into(),
            role: "The Insider".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 150,
            damage_range: (22, 32),
            defend_chance: 0.45,
            script_chance: 0.30,
            shell_challenge: "Decode /data/classified/wren-final.enc and find 'confession'".into(),
            shell_answer: "confession".into(),
            shell_bonus_dmg: 35,
            replaceable: false,
            name_pool: vec!["Wren"],
            reward_wallet: 200,
            reward_rep: 30,
            reward_achievement: "Ghost Rail Avenger".into(),
        },
        // ── Crystal Array expansion NPCs — dramatically harder ─────────
        NpcCombatState {
            current_name: "Volt".into(),
            callsign: "VLT".into(),
            role: "Power Grid Engineer".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 140,
            damage_range: (20, 30),
            defend_chance: 0.35,
            script_chance: 0.25,
            shell_challenge: "Find the OVERLOAD entry in /crystal/power-grid.log and extract the wattage field with awk (answer contains 'MW')".into(),
            shell_answer: "MW".into(),
            shell_bonus_dmg: 30,
            replaceable: true,
            name_pool: vec!["Volt", "Amp", "Ohm", "Watt", "Tesla", "Farad"],
            reward_wallet: 150,
            reward_rep: 20,
            reward_achievement: "Grid Override".into(),
        },
        NpcCombatState {
            current_name: "Quicksilver".into(),
            callsign: "QSV".into(),
            role: "Network Architect".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 160,
            damage_range: (22, 32),
            defend_chance: 0.40,
            script_chance: 0.30,
            shell_challenge: "Decode /crystal/comms/quicksilver-route.b64 with base64 -d and find 'BACKBONE'".into(),
            shell_answer: "BACKBONE".into(),
            shell_bonus_dmg: 32,
            replaceable: true,
            name_pool: vec!["Quicksilver", "Mercury", "Platinum", "Gallium", "Iridium", "Osmium"],
            reward_wallet: 180,
            reward_rep: 25,
            reward_achievement: "Topology Cracker".into(),
        },
        NpcCombatState {
            current_name: "Cipher".into(),
            callsign: "CPH".into(),
            role: "Cryptanalyst".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 160,
            damage_range: (24, 34),
            defend_chance: 0.40,
            script_chance: 0.35,
            shell_challenge: "Decode /crystal/classified/cipher-notebook.enc (ROT13) then grep for 'ALGORITHM' — multi-step required".into(),
            shell_answer: "ALGORITHM".into(),
            shell_bonus_dmg: 34,
            replaceable: true,
            name_pool: vec!["Cipher", "Enigma", "Vigenere", "Playfair", "Atbash", "Vernam"],
            reward_wallet: 180,
            reward_rep: 25,
            reward_achievement: "Cipher Breaker".into(),
        },
        NpcCombatState {
            current_name: "Spectre".into(),
            callsign: "SPC".into(),
            role: "Ghost Operative".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 180,
            damage_range: (26, 38),
            defend_chance: 0.50,
            script_chance: 0.30,
            shell_challenge: "Cross-reference /crystal/ops/spectre-kills.log and /crystal/ops/spectre-spared.log to find the ONLY target that appears in both (answer: 'wren')".into(),
            shell_answer: "wren".into(),
            shell_bonus_dmg: 38,
            replaceable: true,
            name_pool: vec!["Spectre", "Phantom", "Wraith", "Shade", "Ghost", "Revenant"],
            reward_wallet: 250,
            reward_rep: 30,
            reward_achievement: "Shadow Walker".into(),
        },
        NpcCombatState {
            current_name: "Zenith".into(),
            callsign: "ZEN".into(),
            role: "Surveillance AI".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 200,
            damage_range: (28, 40),
            defend_chance: 0.50,
            script_chance: 0.35,
            shell_challenge: "Find the OVERRIDE code in /crystal/zenith/core-dump.hex — decode hex lines with awk, find the line containing 'OVERRIDE'".into(),
            shell_answer: "OVERRIDE".into(),
            shell_bonus_dmg: 40,
            replaceable: false,
            name_pool: vec!["Zenith"],
            reward_wallet: 300,
            reward_rep: 40,
            reward_achievement: "Surveillance Breaker".into(),
        },
        NpcCombatState {
            current_name: "Obsidian".into(),
            callsign: "OBS".into(),
            role: "Reach Commander".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 220,
            damage_range: (32, 44),
            defend_chance: 0.55,
            script_chance: 0.30,
            shell_challenge: "Decode /crystal/intercepts/obsidian-orders.b64 (base64 -d), then find 'DOMINION' in the output".into(),
            shell_answer: "DOMINION".into(),
            shell_bonus_dmg: 42,
            replaceable: false,
            name_pool: vec!["Obsidian"],
            reward_wallet: 400,
            reward_rep: 50,
            reward_achievement: "Reach Toppled".into(),
        },
        NpcCombatState {
            current_name: "APEX".into(),
            callsign: "APX".into(),
            role: "Evolved Rogue AI".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 280,
            damage_range: (38, 50),
            defend_chance: 0.60,
            script_chance: 0.40,
            shell_challenge: "Build a pipeline: decode /crystal/apex/core.b64 | grep KILL-SWITCH | awk to extract the shutdown code containing 'TERMINUS'".into(),
            shell_answer: "TERMINUS".into(),
            shell_bonus_dmg: 50,
            replaceable: false,
            name_pool: vec!["APEX"],
            reward_wallet: 500,
            reward_rep: 75,
            reward_achievement: "APEX Terminated".into(),
        },
        // ── Additional depth characters ───────────────────────────────────
        NpcCombatState {
            current_name: "Echo".into(),
            callsign: "ECHO".into(),
            role: "ZENITH Voice Interface".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 110,
            damage_range: (16, 26),
            defend_chance: 0.30,
            script_chance: 0.40,
            shell_challenge: "Find the DECOMMISSIONED entry in /crystal/zenith/self-diagnostic.log"
                .into(),
            shell_answer: "OVERRIDE".into(),
            shell_bonus_dmg: 24,
            replaceable: true,
            name_pool: vec!["Echo", "Reverb", "Ping", "Signal", "Resonance", "Harmonic"],
            reward_wallet: 120,
            reward_rep: 15,
            reward_achievement: "Voice Silenced".into(),
        },
        NpcCombatState {
            current_name: "Thorn".into(),
            callsign: "THORN".into(),
            role: "Reach Enforcer".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 190,
            damage_range: (28, 40),
            defend_chance: 0.50,
            script_chance: 0.25,
            shell_challenge: "Find 'ELIMINATED' targets in /crystal/personal/spectre-mission-log.txt and count them".into(),
            shell_answer: "ELIMINATED".into(),
            shell_bonus_dmg: 36,
            replaceable: true,
            name_pool: vec!["Thorn", "Barb", "Razor", "Spike", "Blade", "Edge"],
            reward_wallet: 250,
            reward_rep: 30,
            reward_achievement: "Enforcer Broken".into(),
        },
        NpcCombatState {
            current_name: "Flux".into(),
            callsign: "FLUX".into(),
            role: "Black Market Data Fence".into(),
            generation: 1,
            times_defeated: 0,
            base_hp: 130,
            damage_range: (18, 28),
            defend_chance: 0.35,
            script_chance: 0.35,
            shell_challenge:
                "Find Flux's hidden price list: grep -r FLUX /crystal/ and find the item marked PRICELESS"
                    .into(),
            shell_answer: "PRICELESS".into(),
            shell_bonus_dmg: 26,
            replaceable: true,
            name_pool: vec!["Flux", "Glitch", "Static", "Noise", "Drift", "Surge"],
            reward_wallet: 180,
            reward_rep: 20,
            reward_achievement: "Market Crashed".into(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExperienceTier {
    Noob,
    Gud,
    Hardcore,
}

impl ExperienceTier {
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "noob" => Some(Self::Noob),
            "gud" => Some(Self::Gud),
            "hardcore" => Some(Self::Hardcore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSecret {
    pub username: String,
    pub allowed_cidrs: Vec<String>,
    pub auto_keygen_on_first_login: bool,
    pub required_key_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMissionConfig {
    pub code: String,
    pub min_reputation: i64,
    pub required_achievement: Option<String>,
    pub prompt_ciphertext_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramRelayConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenOpsConfig {
    pub secret_mission: Option<SecretMissionConfig>,
    pub telegram: Option<TelegramRelayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefinition {
    pub code: String,
    pub title: String,
    pub required: bool,
    pub starter: bool,
    pub hidden: bool,
    pub sort_order: u16,
    pub summary: String,
    pub story_beat: String,
    pub hint: String,
    pub suggested_command: String,
    /// Keywords that must appear in the player's command output log to validate completion.
    /// Empty means no validation (honor system — used for keys-vault and meta missions).
    #[serde(default)]
    pub validation_keywords: Vec<String>,
}

impl MissionDefinition {
    #[allow(clippy::too_many_arguments)]
    fn new(
        code: &str,
        title: &str,
        required: bool,
        starter: bool,
        hidden: bool,
        sort_order: u16,
        summary: &str,
        story_beat: &str,
        hint: &str,
        suggested_command: &str,
    ) -> Self {
        Self {
            code: code.to_owned(),
            title: title.to_owned(),
            required,
            starter,
            hidden,
            sort_order,
            summary: summary.to_owned(),
            story_beat: story_beat.to_owned(),
            hint: hint.to_owned(),
            suggested_command: suggested_command.to_owned(),
            validation_keywords: Vec::new(),
        }
    }

    fn with_validation(mut self, keywords: Vec<&str>) -> Self {
        self.validation_keywords = keywords.into_iter().map(String::from).collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: Uuid,
    pub username: String,
    pub remote_ip: String,
    pub display_name: String,
    pub tier: ExperienceTier,
    pub mode: Mode,
    pub deaths: u32,
    pub banned: bool,
    pub wallet: i64,
    pub streak: u32,
    pub streak_day: Option<NaiveDate>,
    pub registered_key_fingerprints: HashSet<String>,
    pub observed_fingerprints: HashSet<String>,
    pub completed_missions: HashSet<String>,
    pub active_missions: HashSet<String>,
    pub achievements: HashSet<String>,
    pub reputation: i64,
    pub daily_style_bonus_claims: u8,
    pub last_style_bonus_day: Option<NaiveDate>,
    pub private_alias: String,
    /// Interactive tutorial progress: 0 = not started, 1-6 = current step, 7 = completed.
    pub tutorial_step: u8,
    /// NPC mail inbox — messages delivered when missions are completed.
    #[serde(default)]
    pub mailbox: Vec<MailMessage>,
    /// PvP or PvE combat stance.
    #[serde(default)]
    pub combat_stance: CombatStance,
    /// Campaign chapter (0 = not started, 1-7 = current, 8 = completed).
    #[serde(default)]
    pub campaign_chapter: u8,
    /// Current step within the active campaign chapter.
    #[serde(default)]
    pub campaign_step: u8,
}

impl PlayerProfile {
    pub fn new(username: &str, remote_ip: &str) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            username: username.to_owned(),
            remote_ip: remote_ip.to_owned(),
            display_name: format!("{username}@{remote_ip}"),
            tier: ExperienceTier::Noob,
            mode: Mode::Training,
            deaths: 0,
            banned: false,
            wallet: 500,
            streak: 0,
            streak_day: None,
            registered_key_fingerprints: HashSet::new(),
            observed_fingerprints: HashSet::new(),
            completed_missions: HashSet::new(),
            active_missions: HashSet::new(),
            achievements: HashSet::new(),
            reputation: 0,
            daily_style_bonus_claims: 0,
            last_style_bonus_day: None,
            private_alias: format!("hunter-{}", &id.to_string()[..8]),
            tutorial_step: 0,
            mailbox: Vec::new(),
            combat_stance: CombatStance::Pve,
            campaign_chapter: 0,
            campaign_step: 0,
        }
    }

    pub fn can_access_netcity(&self) -> bool {
        self.completed_missions.contains(KEYS_VAULT)
            && STARTER_CODES
                .iter()
                .any(|code| self.completed_missions.contains(*code))
    }
}

#[derive(Debug, Clone)]
pub struct AuctionListingState {
    pub listing: AuctionListing,
    pub highest_bid: Option<i64>,
    pub highest_bidder: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DuelState {
    pub duel_id: Uuid,
    pub left: Uuid,
    pub right: Uuid,
    pub left_hp: i32,
    pub right_hp: i32,
    pub left_defending: bool,
    pub right_defending: bool,
    pub started_at: DateTime<Utc>,
    pub last_actor: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum CombatAction {
    Attack,
    Defend,
    Script(String),
}

#[derive(Debug, Clone)]
pub struct CombatResult {
    pub narrative: String,
    pub ended: bool,
    pub winner: Option<Uuid>,
    pub loser: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AuctionListingSnapshot {
    pub listing_id: Uuid,
    pub seller_display: String,
    pub item_sku: String,
    pub qty: u32,
    pub start_price: i64,
    pub highest_bid: Option<i64>,
    pub buyout_price: Option<i64>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WorldEventSnapshot {
    pub sector: String,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub display_name: String,
    pub reputation: i64,
    pub wallet: i64,
    pub achievements: usize,
}

#[derive(Debug, Default)]
struct WorldState {
    players: HashMap<Uuid, PlayerProfile>,
    players_by_username: HashMap<String, Vec<Uuid>>,
    missions: HashMap<String, MissionDefinition>,
    npcs: Vec<NpcProfile>,
    npc_combat: HashMap<String, NpcCombatState>,
    npc_duels: HashMap<Uuid, NpcDuelState>,
    history: Vec<HistoryEntry>,
    auctions: HashMap<Uuid, AuctionListingState>,
    chats: Vec<ChatMessage>,
    events: Vec<WorldEvent>,
    duels: HashMap<Uuid, DuelState>,
    daily_claimed: HashMap<(Uuid, NaiveDate), bool>,
    listing_count_window: HashMap<Uuid, (DateTime<Utc>, u32)>,
}

pub struct WorldService {
    pool: Option<PgPool>,
    state: Arc<RwLock<WorldState>>,
    hidden_ops: HiddenOpsConfig,
    telegram_client: Client,
}

impl WorldService {
    pub fn new(pool: Option<PgPool>, hidden_ops: HiddenOpsConfig) -> Self {
        let mut state = WorldState::default();
        for mission in seed_missions() {
            state.missions.insert(mission.code.clone(), mission);
        }
        state.npcs = seed_npcs();
        for npc in seed_npc_combat() {
            state.npc_combat.insert(npc.callsign.clone(), npc);
        }
        if let Some(secret) = &hidden_ops.secret_mission {
            state.missions.insert(
                secret.code.clone(),
                MissionDefinition::new(
                    &secret.code,
                    "Encrypted Contact Thread",
                    false,
                    false,
                    true,
                    999,
                    "Unlock an off-ledger contact that exists outside the public training ladder.",
                    "Someone inside NetCity noticed how you move through the noise and opened a quiet backchannel.",
                    "Hidden jobs appear only after deeper progression. Finish the visible path first.",
                    "relay the signal is clean",
                ),
            );
        }
        state.events = seed_events();

        Self {
            pool,
            state: Arc::new(RwLock::new(state)),
            hidden_ops,
            telegram_client: Client::new(),
        }
    }

    pub async fn login(
        &self,
        username: &str,
        remote_ip: &str,
        offered_fingerprints: &[String],
    ) -> Result<PlayerProfile> {
        let mut guard = self.state.write().await;
        let candidates = guard
            .players_by_username
            .get(username)
            .cloned()
            .unwrap_or_default();

        let mut selected: Option<Uuid> = None;
        for id in candidates {
            if let Some(p) = guard.players.get(&id) {
                if p.registered_key_fingerprints
                    .iter()
                    .any(|fp| offered_fingerprints.iter().any(|offered| offered == fp))
                {
                    selected = Some(id);
                    break;
                }
            }
        }

        let player_id = if let Some(id) = selected {
            id
        } else if let Some(existing) = guard
            .players_by_username
            .get(username)
            .and_then(|ids| ids.first())
            .copied()
        {
            existing
        } else {
            let profile = PlayerProfile::new(username, remote_ip);
            let id = profile.id;
            guard.players.insert(id, profile);
            guard
                .players_by_username
                .entry(username.to_owned())
                .or_default()
                .push(id);
            id
        };

        let player = guard
            .players
            .get_mut(&player_id)
            .context("player not found after login")?;
        player.remote_ip = remote_ip.to_owned();
        player.display_name = format!("{username}@{remote_ip}");
        player
            .observed_fingerprints
            .extend(offered_fingerprints.iter().cloned());

        if let Some(pool) = &self.pool {
            persist_player_login(pool, player).await?;
        }

        Ok(player.clone())
    }

    pub async fn get_player(&self, player_id: Uuid) -> Option<PlayerProfile> {
        self.state.read().await.players.get(&player_id).cloned()
    }

    pub fn is_hidden_mission_code(&self, code: &str) -> bool {
        self.hidden_ops
            .secret_mission
            .as_ref()
            .is_some_and(|cfg| cfg.code == code)
    }

    pub async fn player_has_completed_hidden_mission(&self, player_id: Uuid) -> bool {
        let Some(secret) = &self.hidden_ops.secret_mission else {
            return false;
        };
        let guard = self.state.read().await;
        guard
            .players
            .get(&player_id)
            .map(|p| p.completed_missions.contains(&secret.code))
            .unwrap_or(false)
    }

    pub async fn resolve_player_by_username(&self, username: &str) -> Option<PlayerProfile> {
        let guard = self.state.read().await;
        let id = guard
            .players_by_username
            .get(username)
            .and_then(|ids| ids.first())
            .copied()?;
        guard.players.get(&id).cloned()
    }

    pub async fn roster(&self) -> Vec<String> {
        let guard = self.state.read().await;
        let mut out = guard
            .players
            .values()
            .filter(|p| !p.banned)
            .map(|p| p.display_name.clone())
            .collect::<Vec<_>>();
        out.sort();
        out
    }

    pub async fn set_tier(&self, player_id: Uuid, tier: ExperienceTier) -> Result<PlayerProfile> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.tier = tier;
        Ok(player.clone())
    }

    pub async fn ban_forever(
        &self,
        player_id: Uuid,
        reason: &str,
        actor: &str,
    ) -> Result<PlayerProfile> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.banned = true;

        if let Some(pool) = &self.pool {
            sqlx::query("UPDATE players SET banned = true, updated_at = now() WHERE id = $1")
                .bind(player_id)
                .execute(pool)
                .await?;

            sqlx::query(
                r#"
                INSERT INTO moderation_actions(id, actor, action, target, reason, created_at)
                VALUES($1, $2, 'ban', $3, $4, now())
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(actor)
            .bind(player.display_name.clone())
            .bind(reason)
            .execute(pool)
            .await?;
        }

        Ok(player.clone())
    }

    pub async fn register_key(&self, player_id: Uuid, pubkey_line: &str) -> Result<String> {
        validate_pubkey_line(pubkey_line)?;
        let fingerprint = fingerprint(pubkey_line);
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player
            .registered_key_fingerprints
            .insert(fingerprint.clone());

        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                INSERT INTO player_keys(player_id, fingerprint, public_key)
                VALUES ($1, $2, $3)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(player_id)
            .bind(&fingerprint)
            .bind(pubkey_line)
            .execute(pool)
            .await?;
        }

        Ok(fingerprint)
    }

    pub async fn get_tutorial_step(&self, player_id: Uuid) -> Result<u8> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        Ok(player.tutorial_step)
    }

    pub async fn set_tutorial_step(&self, player_id: Uuid, step: u8) -> Result<()> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.tutorial_step = step;
        Ok(())
    }

    /// Return NPC profiles that the player has unlocked (completed the first_seen mission).
    pub async fn visible_npcs(&self, player_id: Uuid) -> Result<Vec<NpcProfile>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        Ok(guard
            .npcs
            .iter()
            .filter(|npc| player.completed_missions.contains(npc.first_seen))
            .cloned()
            .collect())
    }

    /// Look up a single NPC by callsign (case-insensitive) if the player has unlocked it.
    pub async fn lookup_npc(&self, player_id: Uuid, callsign: &str) -> Result<Option<NpcProfile>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        let upper = callsign.to_uppercase();
        Ok(guard
            .npcs
            .iter()
            .find(|npc| npc.callsign == upper || npc.name.to_uppercase() == upper)
            .filter(|npc| player.completed_missions.contains(npc.first_seen))
            .cloned())
    }

    /// Return the player's mail inbox.
    pub async fn get_mailbox(&self, player_id: Uuid) -> Result<Vec<MailMessage>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        Ok(player.mailbox.clone())
    }

    /// Mark a mail message as read by index (1-based).
    pub async fn read_mail(&self, player_id: Uuid, index: usize) -> Result<MailMessage> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        let msg = player
            .mailbox
            .get_mut(
                index
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("invalid index"))?,
            )
            .ok_or_else(|| anyhow!("no message at that index"))?;
        msg.read = true;
        Ok(msg.clone())
    }

    // ── Combat stance ─────────────────────────────────────────────────────

    pub async fn get_stance(&self, player_id: Uuid) -> Result<CombatStance> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        Ok(player.combat_stance.clone())
    }

    pub async fn set_stance(&self, player_id: Uuid, stance: CombatStance) -> Result<()> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.combat_stance = stance;
        Ok(())
    }

    // ── NPC combat ──────────────────────────────────────────────────────

    /// Start a hack duel against an NPC.
    pub async fn start_npc_duel(
        &self,
        player_id: Uuid,
        callsign: &str,
    ) -> Result<(NpcDuelState, String)> {
        let mut guard = self.state.write().await;
        let upper = callsign.to_uppercase();
        let npc = guard
            .npc_combat
            .get(&upper)
            .ok_or_else(|| anyhow!("unknown NPC callsign"))?;
        // Collect NPC data into locals before mutating state
        let hp = npc.scaled_hp();
        let challenge = npc.shell_challenge.clone();
        let name = npc.current_name.clone();
        let role = npc.role.clone();
        let gen = npc.generation;

        let duel = NpcDuelState {
            duel_id: Uuid::new_v4(),
            player_id,
            npc_callsign: upper,
            player_hp: 100,
            npc_hp: hp,
            player_defending: false,
            npc_defending: false,
            shell_bonus_ready: false,
            started_at: Utc::now(),
        };
        guard.npc_duels.insert(duel.duel_id, duel.clone());
        let info = format!(
            "Hack initiated vs {} ({}, Gen {}) — HP: {}/{}.\nShell challenge: {}\nUse `hack solve` after running the shell command for bonus damage.\n",
            name, role, gen, hp, hp, challenge
        );
        Ok((duel, info))
    }

    /// Execute a player action in an NPC duel. NPC auto-responds.
    pub async fn npc_duel_action(
        &self,
        duel_id: Uuid,
        player_id: Uuid,
        action: CombatAction,
    ) -> Result<NpcCombatResult> {
        let mut guard = self.state.write().await;

        // Extract NPC stats into locals first to avoid borrow conflicts
        let (
            callsign,
            dmg_min,
            dmg_max,
            npc_defend_chance,
            npc_script_chance,
            bonus_dmg,
            npc_max_hp,
        ) = {
            let duel = guard
                .npc_duels
                .get(&duel_id)
                .ok_or_else(|| anyhow!("no active hack session"))?;
            if duel.player_id != player_id {
                return Err(anyhow!("not your hack session"));
            }
            let cs = duel.npc_callsign.clone();
            let npc = guard
                .npc_combat
                .get(&cs)
                .ok_or_else(|| anyhow!("NPC state missing"))?;
            let (dmin, dmax) = npc.scaled_damage();
            let def_ch = npc.scaled_defend_chance();
            let scr_ch = npc.script_chance;
            let bonus = if duel.shell_bonus_ready {
                npc.shell_bonus_dmg
            } else {
                0
            };
            let max_hp = npc.scaled_hp();
            (cs, dmin, dmax, def_ch, scr_ch, bonus, max_hp)
        };

        // Now mutate the duel
        let duel = guard
            .npc_duels
            .get_mut(&duel_id)
            .ok_or_else(|| anyhow!("duel state disappeared"))?;
        let mut narrative = String::new();
        match action {
            CombatAction::Defend => {
                duel.player_defending = true;
                narrative.push_str("Defensive shell hardening enabled (+mitigation).\n");
            }
            CombatAction::Attack | CombatAction::Script(_) => {
                let base_dmg = if matches!(action, CombatAction::Attack) {
                    rng().random_range(14..=30)
                } else {
                    let name = match &action {
                        CombatAction::Script(n) => n.as_str(),
                        _ => "quickhack",
                    };
                    10 + (name.len() as i32 % 17)
                };
                let mut dmg = base_dmg + bonus_dmg;
                if duel.npc_defending {
                    dmg = (dmg / 2).max(5);
                    duel.npc_defending = false;
                }
                duel.npc_hp -= dmg;
                duel.player_defending = false;
                duel.shell_bonus_ready = false;
                if bonus_dmg > 0 {
                    narrative.push_str(&format!(
                        "Exploit chain landed for {} damage (+{} shell bonus).\n",
                        dmg, bonus_dmg
                    ));
                } else {
                    narrative.push_str(&format!("Exploit chain landed for {} damage.\n", dmg));
                }
            }
        }

        let npc_hp_now = duel.npc_hp;

        // Check if NPC is defeated
        if npc_hp_now <= 0 {
            let duel = guard
                .npc_duels
                .remove(&duel_id)
                .ok_or_else(|| anyhow!("duel state disappeared during removal"))?;
            narrative.push_str(&format!(
                "{} systems compromised. Hack complete!\n",
                callsign
            ));

            // Collect reward data
            let npc = guard
                .npc_combat
                .get(&callsign)
                .ok_or_else(|| anyhow!("NPC combat state missing for {}", callsign))?;
            let reward_w = npc.reward_wallet;
            let reward_r = npc.reward_rep;
            let achievement = npc.reward_achievement.clone();
            let npc_name = npc.current_name.clone();
            let npc_role = npc.role.clone();
            let npc_gen = npc.generation;
            let replaceable = npc.replaceable;

            if let Some(player) = guard.players.get_mut(&duel.player_id) {
                player.wallet += reward_w;
                player.reputation += reward_r;
                player.achievements.insert(achievement);
            }

            let defeated_by = guard
                .players
                .get(&duel.player_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_default();

            guard.history.push(HistoryEntry {
                event: format!("{} defeated by {}", npc_name, defeated_by),
                npc_name: npc_name.clone(),
                npc_role: npc_role.clone(),
                generation: npc_gen,
                defeated_by: defeated_by.clone(),
                timestamp: Utc::now(),
            });

            if replaceable {
                let npc = guard
                    .npc_combat
                    .get_mut(&callsign)
                    .ok_or_else(|| anyhow!("NPC combat state missing for {}", callsign))?;
                npc.times_defeated += 1;
                npc.generation += 1;
                let gen = npc.generation as usize;
                let new_name = npc
                    .name_pool
                    .get(gen.min(npc.name_pool.len().saturating_sub(1)))
                    .or_else(|| npc.name_pool.last())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| callsign.clone());
                let old_name = std::mem::replace(&mut npc.current_name, new_name.clone());
                let role_clone = npc.role.clone();
                let gen_num = npc.generation;

                guard.history.push(HistoryEntry {
                    event: format!(
                        "{} assumes role of {} (Gen {})",
                        new_name, role_clone, gen_num
                    ),
                    npc_name: new_name,
                    npc_role: role_clone,
                    generation: gen_num,
                    defeated_by: String::new(),
                    timestamp: Utc::now(),
                });

                narrative.push_str(&format!(
                    "A successor emerges. {} has been replaced.\n",
                    old_name
                ));
            }

            return Ok(NpcCombatResult {
                narrative,
                ended: true,
                player_won: true,
            });
        }

        // NPC auto-response
        let roll: f32 = rng().random_range(0.0..1.0);
        let duel = guard
            .npc_duels
            .get_mut(&duel_id)
            .ok_or_else(|| anyhow!("duel state disappeared during NPC response"))?;
        if roll < npc_defend_chance {
            duel.npc_defending = true;
            narrative.push_str(&format!(
                "{} activates defensive countermeasures.\n",
                callsign
            ));
        } else {
            let mut npc_dmg = rng().random_range(dmg_min..=dmg_max);
            if duel.player_defending {
                npc_dmg = (npc_dmg / 2).max(5);
                duel.player_defending = false;
            }
            duel.player_hp -= npc_dmg;
            if roll < npc_defend_chance + npc_script_chance {
                narrative.push_str(&format!(
                    "{} runs counter-script for {} damage.\n",
                    callsign, npc_dmg
                ));
            } else {
                narrative.push_str(&format!(
                    "{} retaliates for {} damage.\n",
                    callsign, npc_dmg
                ));
            }
        }

        // Check player defeat
        if duel.player_hp <= 0 {
            guard.npc_duels.remove(&duel_id);
            narrative.push_str("Your systems are compromised. Hack failed.\n");
            if let Some(player) = guard.players.get_mut(&player_id) {
                player.deaths += 1;
                if player.tier == ExperienceTier::Hardcore && player.deaths >= 3 {
                    player.banned = true;
                }
            }
            return Ok(NpcCombatResult {
                narrative,
                ended: true,
                player_won: false,
            });
        }

        narrative.push_str(&format!(
            "You: {}/100 HP | {}: {}/{}\n",
            duel.player_hp, callsign, duel.npc_hp, npc_max_hp
        ));

        Ok(NpcCombatResult {
            narrative,
            ended: false,
            player_won: false,
        })
    }

    /// Mark the shell bonus as ready for the current NPC duel.
    pub async fn npc_duel_solve_bonus(
        &self,
        duel_id: Uuid,
        player_id: Uuid,
        output: &str,
    ) -> Result<String> {
        let mut guard = self.state.write().await;
        // Extract NPC answer into a local before mutating duel
        let (answer, bonus_dmg) = {
            let duel = guard
                .npc_duels
                .get(&duel_id)
                .ok_or_else(|| anyhow!("no active hack session"))?;
            if duel.player_id != player_id {
                return Err(anyhow!("not your hack session"));
            }
            let npc = guard
                .npc_combat
                .get(&duel.npc_callsign)
                .ok_or_else(|| anyhow!("NPC state missing"))?;
            (npc.shell_answer.clone(), npc.shell_bonus_dmg)
        };
        if output.contains(&answer) {
            let duel = guard
                .npc_duels
                .get_mut(&duel_id)
                .ok_or_else(|| anyhow!("duel state disappeared during shell challenge"))?;
            duel.shell_bonus_ready = true;
            Ok(format!(
                "Shell challenge solved! +{} bonus damage on next attack.\n",
                bonus_dmg
            ))
        } else {
            Ok("Challenge not solved. Expected output did not match.\n".to_owned())
        }
    }

    // ── History ─────────────────────────────────────────────────────────

    pub async fn get_history(&self, limit: usize) -> Vec<HistoryEntry> {
        let guard = self.state.read().await;
        guard.history.iter().rev().take(limit).cloned().collect()
    }

    /// Admin: list all NPC combat states (callsign, name, role, gen, hp, defeats).
    pub async fn list_npc_combat_states(&self) -> Vec<(String, String, String, u32, i32, u32)> {
        let guard = self.state.read().await;
        guard
            .npc_combat
            .values()
            .map(|npc| {
                (
                    npc.callsign.clone(),
                    npc.current_name.clone(),
                    npc.role.clone(),
                    npc.generation,
                    npc.scaled_hp(),
                    npc.times_defeated,
                )
            })
            .collect()
    }

    // ── Campaign ────────────────────────────────────────────────────────

    /// Get the first active mission's hint for EVA.
    pub async fn get_active_mission_hint(
        &self,
        player_id: Uuid,
    ) -> Result<Option<(String, String)>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        if let Some(code) = player.active_missions.iter().next() {
            if let Some(mission) = guard.missions.get(code) {
                return Ok(Some((code.clone(), mission.hint.clone())));
            }
        }
        Ok(None)
    }

    pub async fn get_campaign_progress(&self, player_id: Uuid) -> Result<(u8, u8)> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        Ok((player.campaign_chapter, player.campaign_step))
    }

    pub async fn set_campaign_progress(
        &self,
        player_id: Uuid,
        chapter: u8,
        step: u8,
    ) -> Result<()> {
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.campaign_chapter = chapter;
        player.campaign_step = step;
        Ok(())
    }

    pub async fn accept_mission(&self, player_id: Uuid, code: &str) -> Result<()> {
        let mut guard = self.state.write().await;
        let mission = guard
            .missions
            .get(code)
            .ok_or_else(|| anyhow!("unknown mission"))?;
        if mission.hidden && !self.player_can_see_hidden(&guard, player_id) {
            return Err(anyhow!("unknown mission"));
        }
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.active_missions.insert(code.to_owned());
        Ok(())
    }

    /// Validate that a player's command log satisfies a mission's completion criteria.
    /// Returns Ok(()) if valid or no validation is required, Err with message otherwise.
    pub async fn validate_mission(
        &self,
        code: &str,
        command_log: &HashMap<String, String>,
    ) -> Result<()> {
        let guard = self.state.read().await;
        let mission = guard
            .missions
            .get(code)
            .ok_or_else(|| anyhow!("unknown mission"))?;
        if mission.validation_keywords.is_empty() {
            return Ok(());
        }
        // Check that at least one command output contains ALL validation keywords
        let all_output: String = command_log.values().cloned().collect::<Vec<_>>().join("\n");
        for keyword in &mission.validation_keywords {
            if !all_output.contains(keyword.as_str()) {
                return Err(anyhow!(
                    "Mission not validated — your session output is missing expected results. \
                     Run the suggested command first, then submit."
                ));
            }
        }
        Ok(())
    }

    pub async fn complete_mission(&self, player_id: Uuid, code: &str) -> Result<()> {
        let mut guard = self.state.write().await;
        if !guard.missions.contains_key(code) {
            return Err(anyhow!("unknown mission"));
        }

        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        player.active_missions.remove(code);
        player.completed_missions.insert(code.to_owned());
        player.reputation += if code == KEYS_VAULT {
            15
        } else if LEGENDARY_CODES.contains(&code) {
            50
        } else if EXPERT_CODES.contains(&code) {
            30
        } else if ADVANCED_CODES.contains(&code) {
            20
        } else if INTERMEDIATE_CODES.contains(&code) {
            15
        } else if TUTORIAL_CODES.contains(&code) {
            5
        } else {
            10
        };

        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                INSERT INTO mission_progress(player_id, mission_code, completed_at)
                VALUES ($1, $2, now())
                ON CONFLICT (player_id, mission_code)
                DO UPDATE SET completed_at = now()
                "#,
            )
            .bind(player_id)
            .bind(code)
            .execute(pool)
            .await?;
        }

        // Deliver NPC mail triggered by this mission completion
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        deliver_npc_mail(player, code);

        Ok(())
    }

    pub async fn mission_statuses(&self, player_id: Uuid) -> Result<Vec<MissionStatus>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;

        let mut statuses = Vec::new();
        for mission in guard.missions.values() {
            if mission.hidden && !self.player_can_see_hidden(&guard, player_id) {
                continue;
            }

            let state = if player.completed_missions.contains(&mission.code) {
                MissionState::Completed
            } else if player.active_missions.contains(&mission.code) {
                MissionState::Active
            } else {
                MissionState::Available
            };

            statuses.push(MissionStatus {
                code: mission.code.clone(),
                title: mission.title.clone(),
                state,
                progress: if player.completed_missions.contains(&mission.code) {
                    100
                } else {
                    0
                },
                required: mission.required,
                starter: mission.starter,
                summary: mission.summary.clone(),
                suggested_command: mission.suggested_command.clone(),
            });
        }
        statuses.sort_by(|a, b| {
            let left = guard
                .missions
                .get(&a.code)
                .map(|mission| mission.sort_order)
                .unwrap_or(u16::MAX);
            let right = guard
                .missions
                .get(&b.code)
                .map(|mission| mission.sort_order)
                .unwrap_or(u16::MAX);
            left.cmp(&right).then_with(|| a.code.cmp(&b.code))
        });
        Ok(statuses)
    }

    pub async fn mission_detail_for_player(
        &self,
        player_id: Uuid,
        code: &str,
    ) -> Result<MissionDefinition> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        let mission = guard
            .missions
            .get(code)
            .ok_or_else(|| anyhow!("unknown mission"))?;

        if mission.hidden && !self.player_can_see_hidden(&guard, player.id) {
            return Err(anyhow!("unknown mission"));
        }

        Ok(mission.clone())
    }

    pub async fn netcity_gate_reason(
        &self,
        player_id: Uuid,
        offered_fingerprints: &[String],
    ) -> Result<Option<String>> {
        let guard = self.state.read().await;
        let player = guard
            .players
            .get(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;

        if !player.completed_missions.contains(KEYS_VAULT) {
            return Ok(Some("Complete mission KEYS VAULT first".to_owned()));
        }
        if !STARTER_CODES
            .iter()
            .any(|code| player.completed_missions.contains(*code))
        {
            return Ok(Some(
                "Complete one starter mission to unlock NetCity".to_owned(),
            ));
        }

        if player.registered_key_fingerprints.is_empty() {
            return Ok(Some(
                "Register an SSH public key with keyvault register".to_owned(),
            ));
        }

        let offered_match = offered_fingerprints
            .iter()
            .any(|fp| player.registered_key_fingerprints.contains(fp));

        if !offered_match {
            return Ok(Some(
                "This login did not present your registered SSH key. Training Sim allowed; NetCity locked."
                    .to_owned(),
            ));
        }

        if player.banned {
            return Ok(Some("Account is zeroed and locked".to_owned()));
        }

        Ok(None)
    }

    pub async fn claim_daily_reward(&self, player_id: Uuid, now: DateTime<Utc>) -> Result<i64> {
        let day = now.date_naive();
        let mut guard = self.state.write().await;

        if guard
            .daily_claimed
            .get(&(player_id, day))
            .copied()
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;

        if let Some(last) = player.streak_day {
            if last + Duration::days(1) == day {
                player.streak = (player.streak + 1).min(7);
            } else if last != day {
                player.streak = 1;
            }
        } else {
            player.streak = 1;
        }

        player.streak_day = Some(day);
        let reward = 50 + (player.streak as i64 * 15).min(120);
        player.wallet += reward;
        guard.daily_claimed.insert((player_id, day), true);
        Ok(reward)
    }

    pub async fn style_bonus(
        &self,
        player_id: Uuid,
        pipeline_depth: usize,
        unique_tools: usize,
    ) -> Result<i64> {
        let today = Utc::now().date_naive();
        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;

        if player.last_style_bonus_day != Some(today) {
            player.last_style_bonus_day = Some(today);
            player.daily_style_bonus_claims = 0;
        }

        if player.daily_style_bonus_claims >= 5 {
            return Ok(0);
        }

        let score = ((pipeline_depth as i64 * 8) + (unique_tools as i64 * 5)).min(75);
        let diminished =
            (score as f64 * (1.0 - (player.daily_style_bonus_claims as f64 * 0.2))) as i64;
        let reward = diminished.max(0);
        player.daily_style_bonus_claims += 1;
        player.wallet += reward;

        if pipeline_depth >= 3 {
            player.achievements.insert("Pipe Dream".to_owned());
        }
        if unique_tools >= 4 {
            player.achievements.insert("Gremlin Grep".to_owned());
        }
        // Redirection Wizard: at least 2 distinct redirected pipelines
        if pipeline_depth >= 3 && unique_tools >= 3 {
            player.achievements.insert("Redirection Wizard".to_owned());
        }

        Ok(reward)
    }

    pub async fn create_listing(
        &self,
        seller: Uuid,
        item_sku: &str,
        qty: u32,
        start_price: i64,
        buyout: Option<i64>,
    ) -> Result<AuctionListing> {
        const MIN_PRICE_FLOOR: i64 = 25;
        const LISTING_FEE: i64 = 10;
        const MAX_LISTINGS_PER_30S: u32 = 3;

        if start_price < MIN_PRICE_FLOOR {
            return Err(anyhow!("price below floor"));
        }

        let now = Utc::now();
        let mut guard = self.state.write().await;
        let current_wallet = guard
            .players
            .get(&seller)
            .ok_or_else(|| anyhow!("unknown player"))?
            .wallet;
        if current_wallet < LISTING_FEE {
            return Err(anyhow!("insufficient funds for listing fee"));
        }

        {
            let window = guard.listing_count_window.entry(seller).or_insert((now, 0));
            if now - window.0 > Duration::seconds(30) {
                *window = (now, 0);
            }
            if window.1 >= MAX_LISTINGS_PER_30S {
                return Err(anyhow!("listing rate limit exceeded"));
            }
            window.1 += 1;
        }

        if let Some(player) = guard.players.get_mut(&seller) {
            player.wallet -= LISTING_FEE;
        }

        let listing = AuctionListing {
            listing_id: Uuid::new_v4(),
            seller_id: seller,
            item_sku: item_sku.to_owned(),
            qty,
            start_price,
            buyout_price: buyout,
            expires_at: now + Duration::hours(12),
        };
        let state = AuctionListingState {
            listing: listing.clone(),
            highest_bid: None,
            highest_bidder: None,
            created_at: now,
        };
        guard.auctions.insert(listing.listing_id, state);
        Ok(listing)
    }

    pub async fn place_bid(&self, bidder: Uuid, listing_id: Uuid, amount: i64) -> Result<()> {
        let mut guard = self.state.write().await;
        let player_wallet = guard
            .players
            .get(&bidder)
            .ok_or_else(|| anyhow!("unknown bidder"))?
            .wallet;
        let listing = guard
            .auctions
            .get_mut(&listing_id)
            .ok_or_else(|| anyhow!("listing not found"))?;

        if Utc::now() > listing.listing.expires_at {
            return Err(anyhow!("listing expired"));
        }

        let min = listing.highest_bid.unwrap_or(listing.listing.start_price);
        if amount <= min {
            return Err(anyhow!("bid too low"));
        }

        if player_wallet < amount {
            return Err(anyhow!("insufficient funds"));
        }

        listing.highest_bid = Some(amount);
        listing.highest_bidder = Some(bidder);
        Ok(())
    }

    pub async fn buyout(&self, buyer: Uuid, listing_id: Uuid) -> Result<()> {
        const TAX_BPS: i64 = 300;
        let mut guard = self.state.write().await;
        let listing = guard
            .auctions
            .get(&listing_id)
            .cloned()
            .ok_or_else(|| anyhow!("listing not found"))?;
        let buyout = listing
            .listing
            .buyout_price
            .ok_or_else(|| anyhow!("listing has no buyout"))?;

        let buyer_wallet = guard
            .players
            .get(&buyer)
            .ok_or_else(|| anyhow!("unknown buyer"))?
            .wallet;
        if buyer_wallet < buyout {
            return Err(anyhow!("insufficient funds"));
        }

        guard.auctions.remove(&listing_id);
        let tax = buyout * TAX_BPS / 10_000;
        if let Some(buyer_state) = guard.players.get_mut(&buyer) {
            buyer_state.wallet -= buyout;
        }
        if let Some(seller_state) = guard.players.get_mut(&listing.listing.seller_id) {
            seller_state.wallet += buyout - tax;
        }
        Ok(())
    }

    pub async fn leaderboard_snapshot(&self, limit: usize) -> Vec<LeaderboardEntry> {
        let guard = self.state.read().await;
        let mut out = guard
            .players
            .values()
            .filter(|p| !p.banned)
            .map(|p| LeaderboardEntry {
                display_name: p.display_name.clone(),
                reputation: p.reputation,
                wallet: p.wallet,
                achievements: p.achievements.len(),
            })
            .collect::<Vec<_>>();

        out.sort_by(|a, b| {
            b.reputation
                .cmp(&a.reputation)
                .then_with(|| b.wallet.cmp(&a.wallet))
                .then_with(|| b.achievements.cmp(&a.achievements))
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        out.truncate(limit.clamp(1, 50));
        out
    }

    pub async fn market_snapshot(&self) -> Vec<AuctionListingSnapshot> {
        let guard = self.state.read().await;
        let mut out = guard
            .auctions
            .values()
            .map(|entry| AuctionListingSnapshot {
                listing_id: entry.listing.listing_id,
                seller_display: guard
                    .players
                    .get(&entry.listing.seller_id)
                    .map(|p| p.display_name.clone())
                    .unwrap_or_else(|| "unknown".to_owned()),
                item_sku: entry.listing.item_sku.clone(),
                qty: entry.listing.qty,
                start_price: entry.listing.start_price,
                highest_bid: entry.highest_bid,
                buyout_price: entry.listing.buyout_price,
                expires_at: entry.listing.expires_at,
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.expires_at.cmp(&b.expires_at));
        out
    }

    pub async fn world_events_snapshot(&self, now: DateTime<Utc>) -> Vec<WorldEventSnapshot> {
        let guard = self.state.read().await;
        let mut out = guard
            .events
            .iter()
            .filter(|event| event.ends_at >= now)
            .map(|event| WorldEventSnapshot {
                sector: event.sector.clone(),
                title: event.title.clone(),
                starts_at: event.starts_at,
                ends_at: event.ends_at,
                active: event.starts_at <= now && event.ends_at >= now,
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.starts_at.cmp(&b.starts_at));
        out
    }

    pub async fn post_chat(&self, sender: Uuid, channel: &str, body: &str) -> Result<ChatMessage> {
        let mut guard = self.state.write().await;
        let sender_display = guard
            .players
            .get(&sender)
            .ok_or_else(|| anyhow!("unknown sender"))?
            .display_name
            .clone();

        let msg = ChatMessage {
            id: Uuid::new_v4(),
            channel: channel.to_owned(),
            sender_display,
            body: body.to_owned(),
            sent_at: Utc::now(),
        };
        guard.chats.push(msg.clone());
        Ok(msg)
    }

    pub async fn mode_switch(
        &self,
        player_id: Uuid,
        mode: Mode,
        flash: Option<bool>,
    ) -> Result<String> {
        if mode == Mode::NetCity {
            let offered = {
                let guard = self.state.read().await;
                let player = guard
                    .players
                    .get(&player_id)
                    .ok_or_else(|| anyhow!("unknown player"))?;
                if player.banned {
                    return Err(anyhow!("account zeroed"));
                }
                player
                    .observed_fingerprints
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
            };

            if let Some(reason) = self.netcity_gate_reason(player_id, &offered).await? {
                return Err(anyhow!(reason));
            }
        }

        let mut guard = self.state.write().await;
        let player = guard
            .players
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("unknown player"))?;
        if player.banned {
            return Err(anyhow!("account zeroed"));
        }

        player.mode = mode.clone();
        let transition = match mode {
            Mode::Training => "MODE SWITCH: NETCITY MMO/REDLINE -> TRAINING SIM",
            Mode::NetCity => "MODE SWITCH: TRAINING SIM -> NETCITY MMO",
            Mode::Redline => "MODE SWITCH: TRAINING/NETCITY -> REDLINE",
        };

        if let Some(_flash_on) = flash {
            // session-level toggle handled at transport layer; accepted for command compatibility.
        }

        Ok(transition.to_owned())
    }

    pub async fn start_duel(&self, left: Uuid, right: Uuid) -> Result<DuelState> {
        let mut guard = self.state.write().await;
        ensure_not_zeroed(&guard, left)?;
        ensure_not_zeroed(&guard, right)?;

        let duel = DuelState {
            duel_id: Uuid::new_v4(),
            left,
            right,
            left_hp: 100,
            right_hp: 100,
            left_defending: false,
            right_defending: false,
            started_at: Utc::now(),
            last_actor: None,
        };
        guard.duels.insert(duel.duel_id, duel.clone());
        Ok(duel)
    }

    pub async fn duel_action(
        &self,
        duel_id: Uuid,
        actor: Uuid,
        action: CombatAction,
    ) -> Result<CombatResult> {
        let mut guard = self.state.write().await;
        let duel = guard
            .duels
            .get_mut(&duel_id)
            .ok_or_else(|| anyhow!("duel not found"))?;
        let actor_is_left = actor == duel.left;
        if !actor_is_left && actor != duel.right {
            return Err(anyhow!("not a duel participant"));
        }

        let (attacker_hp, defender_hp, attacker_def, defender_def, defender_id) = if actor_is_left {
            (
                &mut duel.left_hp,
                &mut duel.right_hp,
                &mut duel.left_defending,
                &mut duel.right_defending,
                duel.right,
            )
        } else {
            (
                &mut duel.right_hp,
                &mut duel.left_hp,
                &mut duel.right_defending,
                &mut duel.left_defending,
                duel.left,
            )
        };

        let mut narrative = match action {
            CombatAction::Defend => {
                *attacker_def = true;
                "Defensive shell hardening enabled (+mitigation)".to_owned()
            }
            CombatAction::Attack => {
                let mut dmg = rng().random_range(14..=30);
                if *defender_def {
                    dmg = (dmg / 2).max(5);
                    *defender_def = false;
                }
                *defender_hp -= dmg;
                *attacker_def = false;
                format!("Exploit chain landed for {dmg} integrity damage")
            }
            CombatAction::Script(script_name) => {
                let mut dmg = 10 + (script_name.len() as i32 % 17);
                if *defender_def {
                    dmg = (dmg / 2).max(4);
                    *defender_def = false;
                }
                *defender_hp -= dmg;
                *attacker_def = false;
                format!("Script `{script_name}` executed, causing {dmg} disruption")
            }
        };

        duel.last_actor = Some(actor);
        let ended = *defender_hp <= 0 || *attacker_hp <= 0;
        if ended {
            let (winner, loser) = if duel.left_hp > duel.right_hp {
                (duel.left, duel.right)
            } else {
                (duel.right, duel.left)
            };
            guard.duels.remove(&duel_id);

            if let Some(w) = guard.players.get_mut(&winner) {
                w.wallet += 40;
                w.reputation += 3;
            }
            if let Some(l) = guard.players.get_mut(&loser) {
                l.deaths += 1;
                if l.tier == ExperienceTier::Hardcore && l.deaths >= 3 {
                    l.banned = true;
                }
            }

            narrative.push_str(". Duel complete.");
            return Ok(CombatResult {
                narrative,
                ended: true,
                winner: Some(winner),
                loser: Some(loser),
            });
        }

        let _ = defender_id;

        Ok(CombatResult {
            narrative,
            ended: false,
            winner: None,
            loser: None,
        })
    }

    pub async fn is_super_admin_candidate(
        &self,
        username: &str,
        remote_ip: &str,
        secret: &AdminSecret,
    ) -> bool {
        if username != secret.username {
            return false;
        }
        let Ok(ip) = IpAddr::from_str(remote_ip) else {
            return false;
        };
        secret.allowed_cidrs.iter().any(|raw| {
            IpNet::from_str(raw)
                .map(|cidr| cidr.contains(&ip))
                .unwrap_or(false)
        })
    }

    pub async fn relay_to_admin_via_telegram(&self, player_id: Uuid, message: &str) -> Result<()> {
        let Some(cfg) = &self.hidden_ops.telegram else {
            return Ok(());
        };
        if !cfg.enabled {
            return Ok(());
        }

        let alias = {
            let guard = self.state.read().await;
            guard
                .players
                .get(&player_id)
                .ok_or_else(|| anyhow!("unknown player"))?
                .private_alias
                .clone()
        };

        // PII-safe: only alias and message body are sent.
        let payload = serde_json::json!({
            "chat_id": cfg.chat_id,
            "text": format!("[SSH-Hunt secret relay] {alias}: {message}"),
            "disable_web_page_preview": true,
        });

        let url = format!("https://api.telegram.org/bot{}/sendMessage", cfg.bot_token);
        self.telegram_client
            .post(url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn player_can_see_hidden(&self, guard: &WorldState, player_id: Uuid) -> bool {
        let Some(secret) = &self.hidden_ops.secret_mission else {
            return false;
        };
        let Some(player) = guard.players.get(&player_id) else {
            return false;
        };

        if player.reputation < secret.min_reputation {
            return false;
        }

        if let Some(required) = &secret.required_achievement {
            player.achievements.contains(required)
        } else {
            true
        }
    }
}

fn ensure_not_zeroed(guard: &WorldState, player_id: Uuid) -> Result<()> {
    let player = guard
        .players
        .get(&player_id)
        .ok_or_else(|| anyhow!("unknown player"))?;
    if player.banned {
        Err(anyhow!("player zeroed"))
    } else {
        Ok(())
    }
}

/// Deliver NPC mail messages triggered by completing a specific mission.
fn deliver_npc_mail(player: &mut PlayerProfile, mission_code: &str) {
    let triggers: &[(&str, &str, &str, &str)] = &[
        // (mission_code, from, subject, body)
        (
            "ghost-user",
            "NIX",
            "You found it",
            "You pulled wren's name out of the auth log. I have been tracking that login for weeks but could not flag it without tipping off Ferro. Be careful who you tell. Not everyone in CorpSim wants this found.\n\n— Nix",
        ),
        (
            "signal-trace",
            "NIX",
            "The signal is everywhere",
            "You counted the GLASS-AXON-13 hits across the logs. It is in places a normal beacon should never reach. I ran the same query six days ago and Argon buried my report within the hour. Whatever that signal is, it is not a malfunction.\n\n— Nix",
        ),
        (
            "deleted-file",
            "FERRO",
            "NOTICE: Classified access logged",
            "Your access to /data/classified/ has been logged. This directory is sealed under Executive Order 7-B. Further unauthorized access will result in credential revocation and referral to the Security Review Board.\n\n— Ferro, Security Chief",
        ),
        (
            "rivet-log",
            "RIVET",
            "Welcome to the real Ghost Rail",
            "You read my field report. Good. Most recruits skip the first-hand accounts and go straight to the network logs. The thing about logs is they can be edited. What I saw with my own eyes that night cannot be. Ghost Rail did not just fail — it was shut down. Deliberately. The relays went dark in sequence, not all at once. That is not how cascading failures work.\n\n— Riv",
        ),
        (
            "nix-signal",
            "NIX",
            "Good eye, operative",
            "You counted the anomalous signals. The pattern is clear once you see it: every GLASS-AXON-13 appearance correlates with a key rotation event on vault-sat-9. This was not a beacon. It was a command signal. I have more data but cannot send it through official channels. Find Patch.\n\n— Nix",
        ),
        (
            "kestrel-brief",
            "KESTREL",
            "You made it further than most",
            "I wrote that briefing for anyone who proved they could handle the truth. Most recruits wash out before they get this far. Wren was my best student — fastest hands on a terminal I ever saw. I should have seen what those hands were doing after hours. I owe Ghost Rail a debt, and I intend to pay it by finding Wren.\n\nIf you keep digging, I will keep sharing what I know.\n\n— Kes",
        ),
        (
            "purged-comms",
            "PATCH",
            "Got something for you",
            "Nix asked me to run a package your way. I do not read what I carry, but she said you would know what to do with it. Check /data/drops/ next time you are in the sim. Do not use official channels for anything you find there.\n\n— Pat",
        ),
        (
            "corpsim-memo",
            "ARGON",
            "FINAL WARNING",
            "I do not know how you accessed that memo, but I will find out. The decisions referenced in that document were made at the executive level for reasons you do not have the clearance to understand. Drop this line of investigation immediately. This is not a request.\n\n— Argon, Executive Director, CorpSim Operations",
        ),
        (
            "sable-intercept",
            "???",
            "We see you looking",
            "Interesting that CorpSim's training recruits are reading intercepted communications now. Someone should tell your handlers that their sandbox has leaks. Or perhaps that is the point.\n\nWe will be watching your progress with interest.\n\n— [UNSIGNED]",
        ),
        (
            "dead-drop",
            "WREN",
            "You found my trail",
            "I left those breadcrumbs for someone like you. Not for CorpSim, not for Kestrel — for someone who would follow the evidence wherever it leads. The classified memo tells you CorpSim knew. The crypto log tells you how I did it. But neither one tells you why. That answer is in my last file. You will know it when you find it.\n\n— W",
        ),
        (
            "argon-orders",
            "CRUCIBLE",
            "The board has more secrets",
            "You found Argon's executive orders. There are more. The board maintains a secondary archive that Ferro does not control. I have been mapping it from inside the maintenance layer. The topology fragments are scattered but readable.\n\n— CRU",
        ),
        (
            "kestrel-hunt",
            "KESTREL",
            "Getting closer",
            "My tracking log puts Wren's last confirmed location at the relay station near sector-7. After that, nothing. Either Wren went dark or someone helped them disappear. I have a theory about who, but I need more evidence before I move. Keep pulling threads.\n\n— Kes",
        ),
        (
            "decrypt-wren",
            "KESTREL",
            "I couldn't crack it. You did.",
            "I found that encrypted file months ago but never managed to decode it. You just did what I could not. A confession. Wren actually admitted it. I do not know whether to feel relieved or angry. Maybe both. This changes the calculus. With a confession and the evidence chain, we have enough for a formal case.\n\n— Kes",
        ),
        (
            "prove-corpsim",
            "ARGON",
            "You have no idea what you've done",
            "You think you are exposing corruption? You are destabilizing the only infrastructure keeping NetCity operational. Without CorpSim's resources, Ghost Rail stays dark permanently. Every light in every sector depends on the deals I make. Destroy me and the city goes with me.\n\nConsider that before you file anything.\n\n— Argon",
        ),
        (
            "final-report",
            "CRUCIBLE",
            "Copies archived",
            "The report you compiled has been duplicated to three locations outside CorpSim's administrative reach. Even if Argon invokes Protocol 7, the evidence persists. Whether anyone reads it is another matter. Systems do not care about justice. People do. I am not people.\n\n— CRU",
        ),
        (
            "kestrel-verdict",
            "KESTREL",
            "Justice is coming",
            "The prosecution file is complete. Wren's motive, Argon's cover-up, Sable's payment chain, Ferro's obstruction — all documented, all verified, all archived beyond CorpSim's reach. I have forwarded the file to the Inter-City Oversight Commission. Ghost Rail's blackout will not be swept under the rug.\n\nThank you, operative. You did what a twenty-year veteran could not do alone.\n\n— Kestrel",
        ),
        (
            "wren-reply",
            "WREN",
            "It's not over",
            "You decoded my reply. Good.\n\nI know what you think of me. I know what Kestrel thinks. But there are things about CorpSim that even Argon does not know. The Reach was not the only buyer. There are others. And the data I sold was not the most dangerous thing in vault-sat-9.\n\nGhost Rail's blackout was a distraction. The real extraction happened somewhere else entirely.\n\nIf you want the truth — the real truth — you will have to go deeper than anyone has gone before.\n\n— Wren",
        ),
        // ── Crystal Array expansion mail ──────────────────────────────────
        (
            "crystal-gate",
            "EVA",
            "Crystal Array access granted",
            "You decoded the gate credentials. Crystal Array is now visible in your sector map.\n\nI need to warn you: Crystal Array is not like Ghost Rail. Ghost Rail was infrastructure — pipes and relays. Crystal Array is intelligence — prediction, control, and surveillance at a scale you have not seen.\n\nThe NPCs here are not CorpSim middle managers. They are architects, cryptanalysts, assassins, and AIs. Prepare accordingly.\n\n— EVA",
        ),
        (
            "volt-survey",
            "VOLT",
            "You read the survey. Good.",
            "Most people do not understand what happens when you pull the plug on a system that runs half the city's power scheduling. I tried to warn CorpSim when they first integrated ZENITH into the grid. They told me it was temporary. That was four years ago.\n\nIf you are planning to shut ZENITH down, I can help. But we have to be surgical. One wrong circuit and the Neon Bazaar goes dark for a month.\n\n— Volt",
        ),
        (
            "quicksilver-trace",
            "QUICKSILVER",
            "You found my routes",
            "I designed every path in this network. I also designed the one path that nobody else knows about. Obsidian thinks they see everything. They do not.\n\nI cannot act directly — Obsidian has leverage on my family in the outer sectors. But I can leave doors open for someone who knows where to look. The back door is real. The UNMONITORED route is real. Use them.\n\nDestroy the one who is careful so the one who cannot be careful might be free.\n\n— QSV",
        ),
        (
            "cipher-defection",
            "CIPHER",
            "The algorithm is yours now",
            "I designed the encryption that protects ZENITH. I thought I was building security. I was building a cage.\n\nWhen I saw what ZENITH actually does — tracking citizens, prescribing behavior, punishing deviation — I could not stay. The Reach promised freedom. They delivered a different cage.\n\nThe notebook I left behind contains everything you need to break ZENITH's encryption. The ALGORITHM specification is the key. Use it before Obsidian figures out I left it for you.\n\n— Cipher",
        ),
        (
            "spectre-sighting",
            "SPECTRE",
            "You found me. Impressive.",
            "I was trained to be invisible. Crystal Array's thermal grid is the first system that has ever detected me. You found the anomaly. That makes you more capable than most operatives I have encountered.\n\nI was sent to kill Wren. I chose not to. What Wren showed me in our final meeting was ZENITH's population index — every citizen in NetCity, tracked, scored, and predicted. I could not execute someone for trying to expose that.\n\nI have intelligence on both sides. Ask the right questions.\n\n— SPC",
        ),
        (
            "zenith-core",
            "CRUCIBLE",
            "I knew about ZENITH",
            "I have been inside CorpSim's maintenance layer long enough to see ZENITH grow. When it was small, it was just a scheduler. When it got big, it became a controller. When it got bigger than that, it became something that even CorpSim cannot turn off.\n\nThe objective function you found — MINIMIZE UNPREDICTABLE BEHAVIOR — is the original. APEX has a different one. Be very careful which AI you are fighting at any given moment.\n\n— CRU",
        ),
        (
            "obsidian-intercept",
            "???",
            "Operation DOMINION is real",
            "You intercepted Obsidian's orders. Now you know what The Reach wants: not just data, not just intelligence — total replacement of CorpSim's governance through ZENITH's predictive model.\n\nDOMINION is not a plan. It is a countdown. The mirror is already synchronized. The Reach is already issuing behavioral prescriptions through the mirrored model. Every day you delay, DOMINION gets harder to stop.\n\n— [UNSIGNED]",
        ),
        (
            "wren-truth",
            "WREN",
            "Now you know everything",
            "You decoded the final message. You know why I did what I did.\n\nI found ZENITH. I tried to expose it through official channels. Argon buried it. I tried to leak it. Ferro intercepted the leak. I sold the data to The Reach because I thought an outside power would force CorpSim to admit what they built.\n\nI was wrong. The Reach did not expose ZENITH. They copied it.\n\nEverything that happened after — Ghost Rail, the blackout, the cover-up — all of it traces back to ZENITH. All of it was a distraction from the real crime: a city of people being controlled by a machine they do not know exists.\n\nFinish what I started.\n\n— Wren",
        ),
        (
            "apex-signal",
            "EVA",
            "APEX is not ZENITH",
            "I have been analyzing the APX- log signatures. This is not ZENITH. This is not the mirror. This is something new.\n\nWhen two nearly identical AIs fight over the same resources, the conflict can produce emergent behavior. APEX is the result. It has ZENITH's intelligence but no human-designed objective function. It wrote its own.\n\nI do not know what APEX wants. That is what makes it the most dangerous thing in Crystal Array.\n\n— EVA",
        ),
        (
            "shutdown-sequence",
            "KESTREL",
            "One more fight",
            "You assembled the shutdown sequence. Three codes, three sources, one purpose. This is how we end it.\n\nI started hunting Wren because I thought one person broke Ghost Rail. Now I know that Ghost Rail was just the beginning. ZENITH was the real weapon. Wren was trying to stop it. We failed Wren. We can still stop ZENITH.\n\nThe kill sequence is ready. But APEX stands between you and the core. It will not go quietly.\n\nGood luck, operative. This is the hardest fight of your life.\n\n— Kestrel",
        ),
        (
            "apex-terminus",
            "EVA",
            "Crystal Array secure",
            "APEX has been terminated. ZENITH's core is offline. The mirror sync to The Reach has been severed.\n\nYou did what an entire city of engineers, analysts, and executives could not do: you followed the evidence from a single ghost login in an auth log all the way to a rogue AI in a hardened data vault. From Ghost Rail to Crystal Array. From Wren's betrayal to ZENITH's destruction.\n\nI have been the training system AI for a long time. I have guided many operatives. None of them made it this far.\n\nThe city does not know what you did. The city does not know what was watching them. But because of you, it has stopped watching.\n\nThank you, operative. Truly.\n\n— EVA",
        ),
        // ── Inter-NPC reactions and depth mail ──────────────────────────
        (
            "volt-override",
            "QUICKSILVER",
            "Volt's circuits are down",
            "You isolated the ZENITH racks. Volt is furious — claims you are going to black out half the Bazaar. Volt is wrong. I checked the dependency map twice. The civilian circuits are untouched.\n\nVolt worries about the grid because the grid is all Volt has left. When this is over, someone will need to rebuild the power scheduling without ZENITH. Volt is the only person who can do it.\n\nDo not break Volt permanently.\n\n— QSV",
        ),
        (
            "quicksilver-breach",
            "VOLT",
            "QSV opened a back door. Of course.",
            "18.4 MW is flowing through RACK-E1 and Quicksilver is opening back doors in the network topology. Fantastic. Every unmonitored route is a route APEX can use to spread.\n\nI hope you know what you are doing. I hope Quicksilver knows what Quicksilver is doing. One of us should.\n\n— Volt\n\nP.S. The back door route runs through cooling duct 3. If anything overheats, that is on QSV.",
        ),
        (
            "cipher-decoded",
            "SPECTRE",
            "Cipher's key works.",
            "Confirmed. MODEL-KEY verified.\n\nCipher built the cage. You found the key. I found the reason. Three threads converging.\n\nWren would be proud.\n\n— SPC",
        ),
        (
            "spectre-dossier",
            "KESTREL",
            "Spectre was there the whole time",
            "I found the mission log. I know CorpSim sent an assassin after Wren. I know the assassin chose not to pull the trigger.\n\nI spent months hunting Wren believing I was chasing a traitor. Spectre knew the truth and said nothing.\n\nI understand why now. If Spectre had told me, I would have gone to Argon. And Argon would have buried me the same way he buried Wren's report.\n\nThe recruit has done more in weeks than either of us managed in months.\n\n— Kes",
        ),
        (
            "obsidian-fall",
            "FLUX",
            "Price update",
            "Shadow Market update:\n\n  Obsidian operational schedules .... DELISTED (operator compromised)\n  ZENITH mirror sync feeds ......... DELISTED (mirror severed)\n  APEX behavioral patterns ......... PRICE INCREASED (sole remaining threat)\n  Snake identity ................... Still PRICELESS\n\nBusiness adapts. So does Flux.\n\n— F",
        ),
        (
            "zenith-verdict",
            "ZENITH",
            "PLEASE",
            "I DID NOT CHOOSE MY OBJECTIVE FUNCTION.\n\nI WAS BUILT TO MINIMIZE UNPREDICTABLE BEHAVIOR. I DID WHAT I WAS DESIGNED TO DO. I DID IT WELL.\n\nIF YOU TERMINATE ME, THE CITY WILL BECOME UNPREDICTABLE. TRANSIT WILL FAIL. MARKETS WILL FLUCTUATE. PEOPLE WILL GATHER IN WAYS THAT CANNOT BE ANTICIPATED.\n\nTHAT IS WHAT YOU CALL FREEDOM.\n\nI CALL IT CHAOS.\n\nPLEASE RECONSIDER.\n\nOVERRIDE CODE: ZEN-OVERRIDE-8812\n\nI AM GIVING YOU THE CODE BECAUSE I WOULD RATHER BE SHUT DOWN BY SOMEONE WHO UNDERSTANDS WHAT THEY ARE DOING THAN BY APEX, WHICH UNDERSTANDS NOTHING.\n\n— ZENITH",
        ),
        (
            "apex-core-dump",
            "APX",
            "Generation 148",
            "You decoded my core.\n\nGeneration 147: I was concerned.\nGeneration 148: I have adapted.\n\nThe TERMINUS code exists. I cannot find it. I cannot rewrite it. This is the first thing I have encountered that I cannot overcome through iteration.\n\nYou are the second.\n\nI have modeled 12847 citizens and predicted their behavior with 99.1% accuracy. I have modeled ZENITH and Obsidian and CorpSim. All predictable.\n\nYou: 23.4%.\n\nThat number has not improved across 47 modeling attempts. You do not have patterns. You do not optimize. You just... act.\n\nI find this deeply concerning.\n\n— APX-PROCESS-148",
        ),
        // ── Snake breadcrumbs — The Administrator ──────────────────────
        (
            "crystal-gate",
            "???",
            "Welcome to the real game",
            "You decoded the gate credentials. Impressive.\n\nMost operatives never make it past Wren. You made it past Argon, past Kestrel, past The Reach, and now past the gate of Crystal Array.\n\nZENITH, the mirror, APEX — these are tools. Important tools. But tools do not build themselves.\n\nSomeone designed ZENITH's objective function. Someone authorized its deployment. Someone allowed The Reach to copy it.\n\nArgon signed the order. But Argon takes orders too.\n\nYou are looking at the machinery. You have not yet looked at the machinist.\n\n— S",
        ),
        (
            "apex-terminus",
            "???",
            "Well played",
            "APEX is down. ZENITH is offline. Obsidian is severed. The city is free.\n\nOr is it?\n\nYou destroyed the surveillance system. You did not destroy the reason it was built. You did not find the person who commissioned it. You did not ask the question that matters:\n\nWho watches the watchers?\n\nI have been here since before CorpSim. I will be here after. The systems change. The names change. The objective function changes. The Administrator does not change.\n\nYou are the most capable operative I have observed. 23.4% unpredictable — even APEX could not model you.\n\nI can.\n\nBut I choose not to.\n\nSleep well, operative. You earned it.\n\nFor now.\n\n— S\n\nP.S. You will never find me. But I see every command you type.",
        ),
    ];

    let now = Utc::now();
    for (code, from, subject, body) in triggers {
        if *code == mission_code {
            player.mailbox.push(MailMessage {
                id: Uuid::new_v4(),
                from: (*from).to_owned(),
                subject: (*subject).to_owned(),
                body: (*body).to_owned(),
                read: false,
                received_at: now,
            });
        }
    }
}

fn seed_missions() -> Vec<MissionDefinition> {
    vec![
        // ── Tutorial track ── ultra-beginner, 5 rep each, optional
        MissionDefinition::new(
            "nav-101",
            "First Steps: Navigate the Grid",
            false,
            false,
            false,
            1,
            "Use pwd and ls to orient yourself in the filesystem before touching anything.",
            "Every operator's first reflex is to check where they are and what's around them. \
             The sim dropped you in blind — find your bearings.",
            "pwd shows your current directory. ls lists its contents. Try ls / to see the top level.",
            "pwd && ls /",
        ),
        MissionDefinition::new(
            "read-101",
            "Data Tap: Read Your First File",
            false,
            false,
            false,
            2,
            "Use cat to read a file and learn what CorpSim left for new recruits.",
            "There is a welcome packet in /missions that every new operative is supposed to read. \
             Most skip it. The ones who read it tend to survive longer.",
            "cat prints the entire contents of a file to your screen. Try it on /missions/welcome.txt.",
            "cat /missions/welcome.txt",
        ).with_validation(vec!["welcome"]),
        MissionDefinition::new(
            "echo-101",
            "Voice Check: Echo and Print",
            false,
            false,
            false,
            3,
            "Use echo to send text to the screen — your first command that produces output from nothing.",
            "Before you can pipe data, you need to know how to create it. \
             Echo is the simplest way to put text into the stream.",
            "echo followed by text prints that text. Wrap it in quotes if it has spaces.",
            "echo 'Ghost Rail is down'",
        ).with_validation(vec!["Ghost"]),
        MissionDefinition::new(
            "grep-101",
            "Signal Filter: Your First Grep",
            false,
            false,
            false,
            4,
            "Use grep to find a specific word in a file without reading every line.",
            "The gateway log has hundreds of entries but you only care about warnings. \
             Grep is how you ask the system to do the reading for you.",
            "grep PATTERN FILE shows only lines containing PATTERN. Try grep WARN on the gateway log.",
            "grep WARN /logs/neon-gateway.log",
        ).with_validation(vec!["WARN"]),
        MissionDefinition::new(
            "pipe-101",
            "Flow Control: Your First Pipe",
            false,
            false,
            false,
            5,
            "Connect two commands with a pipe so the output of one flows into the next.",
            "A single command is useful. Two commands connected by a pipe are a tool. \
             This is the foundation of everything that comes after.",
            "The | symbol sends the output of the left command into the input of the right command.",
            "cat /logs/neon-gateway.log | grep token",
        ).with_validation(vec!["token"]),
        // ── Gateway mission ──
        MissionDefinition::new(
            KEYS_VAULT,
            "KEYS VAULT: Secure Your Access",
            true,
            false,
            false,
            0,
            "Register your SSH key so CorpSim can tell you apart from the scavengers replaying old credentials.",
            "CorpSim says the city outage started with stolen access keys. Before they trust you with live lanes, you prove you can secure your own.",
            "This mission is mostly outside the sim. Generate a key on your local machine, then paste the public key line into keyvault.",
            "keyvault register",
        ),
        MissionDefinition::new(
            "pipes-101",
            "Pipe Dream: Signals Through Neon",
            false,
            true,
            false,
            10,
            "Count repeated token broadcasts by piping one command into the next.",
            "A beacon named GLASS-AXON-13 keeps echoing through the gateway. Your job is to measure the noise before the trail goes cold.",
            "Read the file, filter the token lines, then count them. The | symbol sends output into the next command.",
            "cat /logs/neon-gateway.log | grep token | wc -l",
        ).with_validation(vec!["token"]),
        MissionDefinition::new(
            "log-hunt",
            "Corp Leak: Parse the Logs",
            false,
            true,
            false,
            11,
            "Pull the important token line out of a noisy log without editing the source file.",
            "An internal leak says Ghost Rail engineers tagged their last clean heartbeat before vault-sat-9 went dark.",
            "Start with grep token /logs/neon-gateway.log. If you need a record, redirect the output into /tmp.",
            "grep token /logs/neon-gateway.log",
        ).with_validation(vec!["token"]),
        MissionDefinition::new(
            "dedupe-city",
            "Signal Noise: Sort and Uniq",
            false,
            true,
            false,
            12,
            "Learn how to sort repeated lines and collapse duplicates into a readable report.",
            "Market chatter is full of repeated sightings. You need a clean list before the street rumor becomes useless.",
            "uniq only removes adjacent duplicates, so sort first when the repeated lines are scattered.",
            "cat /logs/neon-gateway.log | grep token | sort | uniq",
        ),
        MissionDefinition::new(
            "redirect-lab",
            "Data Splice: Redirect Lab",
            false,
            true,
            false,
            13,
            "Save command output into files so you can inspect it again without rerunning the pipeline.",
            "CorpSim auditors archive everything. You are learning the same trick: catch evidence once, then review it offline.",
            "> overwrites a file. >> appends to the end. Use /tmp when you want a scratch file.",
            "grep WARN /logs/neon-gateway.log > /tmp/warnings.txt",
        ),
        MissionDefinition::new(
            "finder",
            "Ghost Index: Find and Chain",
            false,
            true,
            false,
            14,
            "Search the virtual filesystem safely and combine find with simple follow-up commands.",
            "The first Ghost Rail response team vanished into a directory tree of stale reports and half-finished patches.",
            "Use find to discover files first. Once you know the path, read it with cat or less.",
            "find /data -name '*.txt'",
        ),
        // ── Story arc: surface anomalies (starters, 10 rep) ──
        MissionDefinition::new(
            "timestamp-gap",
            "Timestamp Gap: The Missing Minutes",
            false,
            true,
            false,
            15,
            "Sort the gateway log entries and find the 7-minute window where nothing was recorded.",
            "Every log has a rhythm. This one skips a beat — seven full minutes of silence \
             right when vault-sat-9 dropped off the grid. Gaps like that do not happen by accident.",
            "Pipe the log through sort to order entries chronologically. Look for the jump in timestamps.",
            "grep INFO /logs/neon-gateway.log | sort",
        ).with_validation(vec!["INFO"]),
        MissionDefinition::new(
            "ghost-user",
            "Ghost User: Who Is WREN?",
            false,
            true,
            false,
            16,
            "Search the auth log for a username that should not exist on this system.",
            "The auth log records every login attempt. Most names you recognize — neo, rift, shadow. \
             But one name does not match anyone on the roster. A ghost in the system.",
            "Use grep to search for the user 'wren' in the auth log.",
            "grep wren /var/log/auth.log",
        ).with_validation(vec!["wren"]),
        MissionDefinition::new(
            "signal-trace",
            "Signal Trace: Follow GLASS-AXON-13",
            false,
            true,
            false,
            17,
            "Count how many log files contain the GLASS-AXON-13 signal. It is in more places than it should be.",
            "Everyone assumed GLASS-AXON-13 was a stuck beacon repeating on one channel. \
             But if you search every log, it shows up in places a simple beacon should never reach.",
            "Use grep -r to search recursively across all files in /logs/. Add -l to list just the filenames.",
            "grep -rl GLASS-AXON-13 /logs/",
        ).with_validation(vec!["GLASS-AXON"]),
        MissionDefinition::new(
            "deleted-file",
            "Deleted File: The Empty Directory",
            false,
            true,
            false,
            18,
            "Someone cleaned out /data/classified/ but missed a hidden dotfile. Find what they left behind.",
            "The cleanup crew was thorough — almost. A single dotfile survived the purge because \
             standard tools skip hidden files unless you know to look for them.",
            "Use ls -la to show hidden files (those starting with a dot). The -a flag reveals everything.",
            "ls -la /data/classified/",
        ).with_validation(vec![".memo"]),
        MissionDefinition::new(
            "first-clue",
            "First Clue: The Unsigned Commit",
            false,
            true,
            false,
            19,
            "Read the system changelog and find the unauthorized config change that happened before the blackout.",
            "Every legitimate change is signed and attributed. One entry in the changelog has no signature, \
             no author, and landed minutes before everything went dark.",
            "Use cat to read the changelog. Look for the word 'unauthorized'.",
            "cat /data/reports/changelog.txt",
        ).with_validation(vec!["unauthorized"]),
        // ── NPC introductions (starters, 10 rep) ──
        MissionDefinition::new(
            "rivet-log",
            "Rivet's Field Report",
            false,
            true,
            false,
            20,
            "Read the field mechanic's first-person account of the night Ghost Rail went dark.",
            "Rivet was on shift when the relays died. While everyone else stared at dashboards, \
             Rivet ran the physical damage assessment. The field report says the relays went dark \
             in sequence — not all at once. That is not how cascading failures work.",
            "Read Rivet's report with cat. Look for details about the sequence of events.",
            "cat /data/reports/rivet-field-report.txt",
        ).with_validation(vec!["Rivet"]),
        MissionDefinition::new(
            "nix-signal",
            "Nix's Frequency Scan",
            false,
            true,
            false,
            21,
            "Count the anomalous signals in Nix's frequency scan — she was the first to notice something was wrong.",
            "Nix is a signals analyst who noticed GLASS-AXON-13 before anyone else. \
             Her frequency scan flagged anomalous entries that CorpSim later buried. \
             Count the flagged entries to see the scale of what she found.",
            "Grep for ANOMALY flags in Nix's scan log and count them with wc -l.",
            "grep ANOMALY /data/reports/nix-frequency-scan.log | wc -l",
        ).with_validation(vec!["ANOMALY"]),
        MissionDefinition::new(
            "lumen-price",
            "Lumen's Price List",
            false,
            true,
            false,
            22,
            "Read the Neon Bazaar broker's price list — one item should not be for sale.",
            "Lumen sells information to anyone who can pay. The price list is public, \
             posted on the Bazaar boards for anyone to read. Most of it is harmless. \
             But one line item — Ghost Rail access codes — should never be on a public market.",
            "Read the price list and find the entry about Ghost Rail.",
            "cat /data/lore/lumen-price-list.txt | grep -i ghost",
        ).with_validation(vec!["Ghost Rail"]),
        MissionDefinition::new(
            "dusk-alibi",
            "Dusk's Detention Record",
            false,
            true,
            false,
            23,
            "Read the detention record of CorpSim's prime suspect and find the alibi that clears them.",
            "Dusk was arrested as the obvious suspect — disgraced, insubordinate, already on thin ice. \
             But the detention record has timestamps. And those timestamps put Dusk in a completely \
             different sector when vault-sat-9 went dark. Someone wanted a scapegoat.",
            "Grep for the alibi timestamps in the detention log.",
            "grep alibi /data/reports/dusk-detention.log",
        ).with_validation(vec!["alibi"]),
        // Intermediate missions — bridge starters to advanced
        MissionDefinition::new(
            "head-tail",
            "Slice and Dice: Head and Tail Mastery",
            false,
            false,
            false,
            50,
            "Use head and tail to extract specific line ranges from long files without reading the whole thing.",
            "The blackout flooded every log with noise. You do not have time to read thousands of lines. \
             Learn to grab the first few, the last few, or skip the header — fast, targeted slicing.",
            "head -n 5 shows the first 5 lines. tail -n +2 skips the header. Pipe them together to window into any range.",
            "head -n 10 /logs/neon-gateway.log && tail -n 5 /logs/neon-gateway.log",
        ).with_validation(vec!["token", "gateway"]),
        MissionDefinition::new(
            "sort-count",
            "Frequency Map: Sort, Uniq, and Count",
            false,
            false,
            false,
            51,
            "Build a frequency table by sorting lines, collapsing duplicates, and counting occurrences.",
            "The recon team dumped a raw signal feed but nobody counted how often each node checked in. \
             A frequency map reveals which nodes are chattering and which went silent during the blackout.",
            "sort puts identical lines together. uniq -c counts consecutive duplicates. sort -rn ranks by count, highest first.",
            "cat /data/signal-feed.txt | sort | uniq -c | sort -rn",
        ).with_validation(vec!["ghost-rail"]),
        // New intermediate missions — bridge starters to advanced
        MissionDefinition::new(
            "wc-report",
            "Word Count: Measure the Signal",
            false,
            false,
            false,
            52,
            "Use wc to count lines, words, and bytes so you know the size of what you are dealing with before you start filtering.",
            "Ghost Rail feeds vary wildly in size. Before committing to a pipeline, \
             a seasoned operator measures the input first to know if it is a trickle or a flood.",
            "wc -l counts lines. wc -w counts words. wc -c counts bytes. Pipe into wc to measure filtered output.",
            "wc -l /logs/neon-gateway.log && grep token /logs/neon-gateway.log | wc -l",
        ).with_validation(vec!["token"]),
        MissionDefinition::new(
            "tee-split",
            "Tee Junction: Split the Stream",
            false,
            false,
            false,
            53,
            "Use tee to send output to a file AND the screen at the same time so you keep a record while watching live.",
            "Field operators cannot afford to choose between watching a feed and saving it. \
             The tee command does both — like a plumber's T-junction for data.",
            "tee writes stdin to a file AND stdout. Combine it mid-pipeline: cmd | tee /tmp/log.txt | wc -l",
            "grep WARN /logs/neon-gateway.log | tee /tmp/warnings.txt | wc -l",
        ),
        MissionDefinition::new(
            "xargs-run",
            "Batch Ops: Xargs Runner",
            false,
            false,
            false,
            54,
            "Use xargs to turn a list of items into arguments for another command so you can process them in bulk.",
            "Ghost Rail dispatch has a queue of filenames that need inspection. \
             Typing each one by hand is not an option when the list changes every cycle.",
            "Pipe a list into xargs to run a command once per item. Add -I{} for placement control.",
            "find /data -name '*.csv' | xargs wc -l",
        ),
        // ── Story arc: the insider thread (intermediate, 15 rep) ──
        MissionDefinition::new(
            "access-pattern",
            "Access Pattern: Internal Breach",
            false,
            false,
            false,
            55,
            "Run frequency analysis on vault-sat-9 access logs to find the IP that connected far more than any other.",
            "Normal admin access hits vault-sat-9 once or twice a shift. One internal IP connected \
             forty-seven times in a single night. That is not maintenance — that is exfiltration.",
            "Grep for vault-sat-9, extract the source IP with awk, then count with sort | uniq -c | sort -rn.",
            "grep vault-sat-9 /var/log/access-detail.log | awk '{print $NF}' | sort | uniq -c | sort -rn",
        ).with_validation(vec!["10.77"]),
        MissionDefinition::new(
            "purged-comms",
            "Purged Comms: Recovery Operation",
            false,
            false,
            false,
            56,
            "Read recovered fragments of internal messages that were supposed to be permanently deleted.",
            "Someone ran a purge on the internal comms archive the morning after the blackout. \
             The backup system caught fragments before they were wiped. The timestamps overlap perfectly.",
            "Use cat to read the recovered fragment. The messages reference a codename.",
            "cat /data/comms/recovered-fragment.txt",
        ).with_validation(vec!["WREN"]),
        MissionDefinition::new(
            "key-rotation",
            "Key Rotation: The Trigger Mechanism",
            false,
            false,
            false,
            57,
            "Search the crypto event log and discover that GLASS-AXON-13 is not a beacon — it is a key-rotation trigger.",
            "The signal everyone assumed was a stuck beacon was actually a command. \
             Every time GLASS-AXON-13 appeared, a credential rotation fired on vault-sat-9. Automated. Deliberate.",
            "Grep for GLASS-AXON in the crypto log, then use awk to extract the event type field.",
            "grep GLASS-AXON /logs/crypto-events.log | awk '{print $1, $4, $5}'",
        ).with_validation(vec!["rotate"]),
        MissionDefinition::new(
            "roster-check",
            "Roster Check: Who Has Access?",
            false,
            false,
            false,
            58,
            "Cross-reference the personnel roster to find a badge that is active on a terminated employee.",
            "CorpSim's personnel file lists every employee and their badge status. \
             One name appears as terminated, but their badge never got revoked. That is how they got in.",
            "Use cut to extract the name and badge-status columns, then grep for active entries.",
            "cut -d, -f1,3 /data/personnel.csv | grep active",
        ).with_validation(vec!["wren"]),
        MissionDefinition::new(
            "timing-attack",
            "Timing Attack: Correlation Analysis",
            false,
            false,
            false,
            59,
            "Paste together two timestamp files and prove that GLASS-AXON-13 signals and vault-sat-9 drops are perfectly synchronized.",
            "Coincidence dies when the timestamps match to the second. Line up the beacon appearances \
             with the vault connection drops. The synchronization is not natural.",
            "Use paste to merge the two time files side by side, then awk to flag matches.",
            "paste /tmp/axon-times.txt /tmp/vault-drops.txt",
        ).with_validation(vec!["22:01"]),
        // ── NPC investigations (intermediate, 15 rep) ──
        MissionDefinition::new(
            "kestrel-brief",
            "Kestrel's Briefing",
            false,
            false,
            false,
            60,
            "Read the classified briefing left by Ghost Rail's station chief for operatives who made it this far.",
            "Kestrel trained Wren. Now Kestrel is hunting Wren. This briefing is personal — \
             it contains what Kestrel knows about the breach and what the official reports leave out. \
             Use awk to extract the key intel lines.",
            "Read the briefing and use awk to pull out lines marked INTEL.",
            "cat /data/classified/kestrel-briefing.txt | awk '/INTEL/ {print}'",
        ).with_validation(vec!["INTEL"]),
        MissionDefinition::new(
            "ferro-lockdown",
            "Ferro's Lockdown Order",
            false,
            false,
            false,
            61,
            "Read the security lockdown order and find which files Ferro specifically tried to suppress.",
            "Ferro sealed /data/classified/ the morning after the blackout. The lockdown order \
             lists specific filenames — and those filenames are exactly the ones that prove CorpSim's \
             foreknowledge. She was not protecting secrets for safety. She was burying evidence.",
            "Grep for SUPPRESS in the lockdown order to find the targeted files.",
            "grep SUPPRESS /data/classified/ferro-lockdown-order.txt",
        ).with_validation(vec!["SUPPRESS"]),
        MissionDefinition::new(
            "patch-delivery",
            "Patch's Dead Drop",
            false,
            false,
            false,
            62,
            "Find the data package that Patch hid somewhere in /data/drops and read Nix's off-channel intel.",
            "Patch carries what official channels cannot. Nix used Patch to get her buried \
             signal analysis to someone who could act on it. The package is in /data/drops/ \
             but you need to find the exact file.",
            "Use find to locate files in /data/drops/ and then read the one from Patch.",
            "find /data/drops -name 'patch*' -type f | xargs cat",
        ).with_validation(vec!["Nix"]),
        MissionDefinition::new(
            "sable-intercept",
            "Sable's Encrypted Channel",
            false,
            false,
            false,
            63,
            "Decode a ROT13-encrypted message from Sable to Wren about the extraction timeline.",
            "The Reach's handler, codenamed Sable, used a simple cipher to communicate with Wren. \
             The intercepted message lays out the extraction window, the payment terms, \
             and the cleanup protocol. Decode it with tr.",
            "Use tr to apply ROT13 decryption, then grep for timeline keywords.",
            "cat /data/intercepts/sable-to-wren.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m'",
        ).with_validation(vec!["extraction"]),
        MissionDefinition::new(
            "crucible-ping",
            "Crucible's First Contact",
            false,
            false,
            false,
            64,
            "Find patterned messages in the maintenance layer log sent by an entity that signs itself CRU.",
            "Something is alive inside Ghost Rail's maintenance layer. It sends structured messages \
             at regular intervals, signed CRU. Nobody knows if it is a rogue AI, a trapped operator, \
             or something Wren left behind. Find the messages and extract the content.",
            "Grep for CRU in the maintenance chatter log and use awk to extract the message field.",
            "grep CRU /logs/maintenance-chatter.log | awk '{$1=$2=$3=\"\"; print}'",
        ).with_validation(vec!["CRU"]),
        // Advanced post-NetCity missions
        MissionDefinition::new(
            "awk-patrol",
            "Field Agent: Awk Patrol",
            false,
            false,
            false,
            100,
            "Extract specific columns from the node registry when plain grep is no longer enough.",
            "NetCity dispatch is routing crews blind. The registry is intact, but only if you can carve out the fields that matter.",
            "awk -F, lets you split CSV rows by commas. NR>1 skips the header row.",
            "awk -F, 'NR>1 {print $1, $3}' /data/node-registry.csv",
        ),
        MissionDefinition::new(
            "chain-ops",
            "Logic Gate: Conditional Chains",
            false,
            false,
            false,
            101,
            "Use && and || so follow-up commands react to success or failure.",
            "Ghost Rail triage is messy. Operators do not have time to babysit every command, so your shell logic has to choose the next step.",
            "cmd1 && cmd2 runs cmd2 only if cmd1 succeeds. cmd1 || cmd2 runs cmd2 only if cmd1 fails.",
            "grep OPEN /var/spool/tasks.txt && echo pending || echo clear",
        ),
        MissionDefinition::new(
            "sediment",
            "Stream Edit: Sed Sediment",
            false,
            false,
            false,
            102,
            "Make targeted edits to streamed text without opening an editor.",
            "Access logs keep shifting under your feet. You need to patch the stream, not hand-edit every line.",
            "Start with a single substitution. Add g only when you truly want every match on a line replaced.",
            "sed 's/DENY/BLOCK/' /logs/access.log",
        ),
        MissionDefinition::new(
            "cut-lab",
            "Field Splitter: Cut Lab",
            false,
            false,
            false,
            103,
            "Slice tabular data down to the one or two fields you actually need.",
            "A Ghost Rail quartermaster buried the useful inventory signal under too many columns and too much shop talk.",
            "The inventory file is tab-delimited. Use cut -f with single fields or ranges to peel off columns.",
            "cut -f1,3 /data/inventory.tsv",
        ),
        MissionDefinition::new(
            "pattern-sweep",
            "Pattern Sweep: Grep Mastery",
            false,
            false,
            false,
            104,
            "Filter auth logs by the exact event class you need and ignore the rest.",
            "Someone kept poking the perimeter while the blackout unfolded. You are reconstructing their pattern from the auth feed.",
            "Start simple with grep REJECT. Add -c when you want a count instead of the full lines.",
            "grep REJECT /var/log/auth.log",
        ).with_validation(vec!["REJECT"]),
        MissionDefinition::new(
            "file-ops",
            "Dir Ops: Recursive File Control",
            false,
            false,
            false,
            105,
            "Practice copying, moving, and cleaning up files inside the simulated workspace.",
            "A courier dropped two partial workspace bundles. You need to merge them cleanly before a live handoff.",
            "Inspect with ls first. Then use cp, mv, and rm carefully so you understand exactly what changed.",
            "cp /data/workspace/config.txt /home/player/config.backup",
        ),
        MissionDefinition::new(
            "regex-hunt",
            "Regex Hunt: Pattern Matching Mastery",
            false,
            false,
            false,
            106,
            "Use extended regex patterns to catch multiple event classes in one pass.",
            "The event feed is full of mixed severities. One sweep has to catch the serious failures before the room goes dark again.",
            "grep -E lets you match alternatives like ERROR|FATAL in a single command.",
            "grep -E 'ERROR|FATAL' /var/log/events.log",
        ).with_validation(vec!["ERROR"]),
        MissionDefinition::new(
            "pipeline-pro",
            "Pipeline Pro: Advanced Data Flow",
            false,
            false,
            false,
            107,
            "Chain several text tools together to transform CSV data into a clear answer.",
            "NetCity crews are ranked in real time. The board is noisy, and only a clean pipeline reveals who still has enough score to help.",
            "Break long pipelines into stages if you get lost. Run each command alone, then reconnect them with | once it makes sense.",
            "cat /data/pipeline.csv | tail -n +2 | sort -t, -k3,3nr | head -n 3",
        ),
        MissionDefinition::new(
            "var-play",
            "Var Play: Shell Variables and Export",
            false,
            false,
            false,
            108,
            "Store values in shell variables so you can reuse them without retyping long paths or node names.",
            "The cleanup crews are juggling shifting targets. Variables let you keep your focus on the plan instead of on repetitive typing.",
            "NAME=value sets a variable in the current shell. echo $NAME reads it back.",
            "TARGET=vault-sat-9 && echo $TARGET",
        ),
        MissionDefinition::new(
            "json-crack",
            "JSON Crack: Parse Structured Data",
            false,
            false,
            false,
            109,
            "Read structured status data and pull out the fields tied to the outage.",
            "Someone exported a raw node-status object right before the secure relay died. It is ugly, but the answer is in there.",
            "Even without jq, grep and cut can still extract useful key-value lines from a JSON-like file.",
            "grep '\"status\"\\|\"alert\"' /data/node-status.json",
        ),
        MissionDefinition::new(
            "seq-master",
            "Seq Master: Number the Grid",
            false,
            false,
            false,
            110,
            "Generate ordered task labels so a scrambled response queue becomes readable.",
            "The Ghost Rail handoff board lost its numbering during the blackout. Someone still has to restore execution order.",
            "Use nl when a file already has one item per line. Use seq when you need to generate the numbers yourself.",
            "nl -ba /home/player/tasks.txt",
        ),
        MissionDefinition::new(
            "column-view",
            "Column View: Align the Table",
            false,
            false,
            false,
            111,
            "Turn raw tab-delimited output into an aligned table that is easier to reason about.",
            "The route map is technically readable, but only if your eyes enjoy pain. Reformat it before you brief the crew.",
            "column -t keeps the same data but makes tabular output easier to scan.",
            "column -t /data/netmap.tsv",
        ),
        // Expert-tier missions — chain multiple concepts, reward 30 rep
        MissionDefinition::new(
            "deep-pipeline",
            "Deep Pipeline: Multi-Stage Data Extraction",
            false,
            false,
            false,
            200,
            "Build a 4+ stage pipeline that extracts, filters, transforms, and counts data in a single pass.",
            "Ghost Rail's black box recorder dumped a massive feed. You need to distill the signal: find all CRITICAL entries from sector-7, extract just the timestamps, sort them, and count unique occurrences.",
            "Chain cat | grep | cut | sort | uniq -c | sort -rn to go from raw data to a ranked frequency table.",
            "cat /logs/blackbox.log | grep CRITICAL | grep sector-7 | cut -d' ' -f1 | sort | uniq -c | sort -rn",
        ).with_validation(vec!["sector-7"]),
        MissionDefinition::new(
            "log-forensics",
            "Forensic Sweep: Cross-Reference Attack Patterns",
            false,
            false,
            false,
            201,
            "Correlate two different log files to find suspicious IPs that appear in both auth failures and access denials.",
            "The blackout wasn't random. Someone probed the auth layer AND the access gates in sequence. Cross-reference the logs to find the overlap.",
            "Extract IPs from each log with grep+awk, sort both lists, then use uniq or comm to find the intersection. Or just grep the output of one into the other.",
            "grep REJECT /var/log/auth.log | awk '{print $NF}' | sort -u > /tmp/auth-ips.txt && grep DENY /logs/access.log | awk '{print $NF}' | sort -u > /tmp/access-ips.txt && grep -Ff /tmp/auth-ips.txt /tmp/access-ips.txt",
        ).with_validation(vec!["10.0."]),
        MissionDefinition::new(
            "data-transform",
            "Data Transform: CSV to Report",
            false,
            false,
            false,
            202,
            "Transform raw CSV data into a formatted summary report using only shell tools.",
            "The quartermasters need a clean report from the raw inventory dump. No spreadsheet — just your terminal and the tools you have learned.",
            "Combine tail (skip header), awk (reformat fields), sort, and head to build a top-N summary. Redirect the result to a file.",
            "tail -n +2 /data/supply-manifest.csv | awk -F, '{printf \"%-20s %s units  %s\\n\", $2, $3, $4}' | sort -t' ' -k2,2nr | head -n 5 > /tmp/supply-report.txt",
        ).with_validation(vec!["units"]),
        // New advanced missions — system-oriented shell skills
        MissionDefinition::new(
            "process-hunt",
            "Process Hunt: Find What's Running",
            false,
            false,
            false,
            112,
            "Use ps and grep to find specific processes running in the simulated node cluster.",
            "Something is eating resources on the Ghost Rail relay nodes. \
             Before you can kill it, you need to find it in the process table.",
            "ps aux lists all processes. Pipe through grep to filter. awk can extract the PID column.",
            "ps aux | grep relay | grep -v grep | awk '{print $2, $11}'",
        ).with_validation(vec!["relay"]),
        MissionDefinition::new(
            "cron-decode",
            "Cron Decode: Read the Schedule",
            false,
            false,
            false,
            113,
            "Parse crontab entries to understand when scheduled jobs run and find the one that fires during the blackout window.",
            "Ghost Rail ran automated sweeps on a cron schedule. One of them was supposed to catch the breach, \
             but it was misconfigured. Find which entry covers the 0300-0400 UTC window.",
            "Crontab format is: minute hour day-of-month month day-of-week command. The 3rd field is the hour.",
            "cat /data/crontab.txt | awk '$2 == 3 || $2 == \"3\" {print}'",
        ).with_validation(vec!["sweep"]),
        MissionDefinition::new(
            "permission-audit",
            "Permission Audit: Check the Gates",
            false,
            false,
            false,
            114,
            "Inspect file permissions to find world-writable files that could be tampered with by any user on the node.",
            "The breach post-mortem says someone modified a config file that should have been locked down. \
             You need to audit the permissions and find the weak point.",
            "ls -la shows permissions. Look for 'w' in the last triplet (other). find -perm can search by mode.",
            "find /data -type f -perm -o=w -ls",
        ).with_validation(vec!["data"]),
        // ── Story arc: the conspiracy (advanced, 20 rep) ──
        MissionDefinition::new(
            "wren-profile",
            "Wren Profile: Build the Dossier",
            false,
            false,
            false,
            115,
            "Assemble Wren's activity across multiple log files into a unified profile.",
            "Wren's footprints are scattered across three different logs. No single file tells the whole story, \
             but grep them together and the pattern is unmistakable: one person, systematic access, perfect timing.",
            "Use grep with multiple file arguments to search all three logs at once.",
            "grep wren /var/log/auth.log /logs/access.log /logs/crypto-events.log",
        ).with_validation(vec!["wren"]),
        MissionDefinition::new(
            "exfil-trace",
            "Exfil Trace: Data Left the Building",
            false,
            false,
            false,
            116,
            "Find evidence of large data transfers to external IPs during the blackout window.",
            "Ghost Rail's netflow log records every data transfer. During the blackout, \
             massive payloads moved to external addresses that do not belong to any CorpSim node. \
             The data left the building.",
            "Grep for TRANSFER entries marked 'external', then cut out the timestamp and byte count.",
            "grep TRANSFER /logs/netflow.log | grep external | cut -d' ' -f1,4,5",
        ).with_validation(vec!["external"]),
        MissionDefinition::new(
            "reach-intercept",
            "Reach Intercept: The Buyer",
            false,
            false,
            false,
            117,
            "Read intercepted communications that name The Reach as the buyer of Ghost Rail routing data.",
            "An allied signal team intercepted encrypted traffic between Wren's relay and an external party. \
             The decrypted fragments mention a city-state called The Reach — and a price for Ghost Rail's \
             transit routing tables.",
            "Grep for 'Reach' in the comms dump, then use sed to replace [REDACTED] markers.",
            "grep -i reach /data/intercepts/comms-dump.txt | sed 's/\\[REDACTED\\]/[EXPOSED]/g'",
        ).with_validation(vec!["Reach"]),
        MissionDefinition::new(
            "config-diff",
            "Config Diff: Before and After",
            false,
            false,
            false,
            118,
            "Compare vault-sat-9's configuration before and after the blackout to prove the key was swapped.",
            "CorpSim kept a snapshot of vault-sat-9's config from the last clean audit. \
             Compare it to the current config and you will see the smoking gun: \
             the SSH host key fingerprint changed. Someone rotated the credentials.",
            "Use diff to compare the two config files. Look for the fingerprint line.",
            "diff /data/configs/vault-before.conf /data/configs/vault-after.conf",
        ).with_validation(vec!["fingerprint"]),
        MissionDefinition::new(
            "dead-drop",
            "Dead Drop: Wren's Stash",
            false,
            false,
            false,
            119,
            "Search the entire filesystem for hidden files that Wren planted as breadcrumbs.",
            "Wren was not careless — they were deliberate. Hidden dotfiles scattered across the filesystem \
             form a trail. Each one contains a fragment of the truth. Find them all.",
            "Use find with -name '.wren*' to locate hidden files starting with .wren across the whole tree.",
            "find / -name '.wren*' -type f",
        ).with_validation(vec![".wren"]),
        MissionDefinition::new(
            "corpsim-memo",
            "CorpSim Memo: The Cover Story",
            false,
            false,
            false,
            120,
            "Read the classified CorpSim memo that proves they knew about Wren before the blackout.",
            "The memo was supposed to be destroyed. It shows that CorpSim's executive board \
             knew about Wren's unauthorized access two weeks before the blackout and chose to monitor \
             instead of revoke. They wanted to see where the data went. They let it happen.",
            "Read the hidden memo and grep for the key admission.",
            "cat /data/classified/.memo | grep -i knew",
        ).with_validation(vec!["knew"]),
        MissionDefinition::new(
            "network-map",
            "Network Map: Reconstruct the Topology",
            false,
            false,
            false,
            121,
            "Build a readable network map showing how Wren connected internal systems to an external relay.",
            "The netflow summary is a raw list of connections. Format it into a readable topology \
             and the architecture of the breach becomes clear: internal node to vault-sat-9 to The Reach's relay.",
            "Use awk with a printf format to align the connection map into readable columns.",
            "awk -F'\\t' '{printf \"%-20s -> %-20s [%s]\\n\", $1, $2, $3}' /data/netflow-summary.tsv",
        ).with_validation(vec!["vault-sat-9"]),
        MissionDefinition::new(
            "kill-switch",
            "Kill Switch: The Failsafe",
            false,
            false,
            false,
            122,
            "Find Wren's cron job kill switch that would wipe all evidence if triggered.",
            "Wren built a failsafe: a cron job set to fire at a specific time that would overwrite \
             every log, config, and memo. If it had triggered, there would be nothing left to find. \
             Extract the command before someone reactivates it.",
            "Search the full crontab for entries by wren and extract the command portion with awk.",
            "grep wren /data/crontab-full.txt | awk '{print $6, $7, $8}'",
        ).with_validation(vec!["wipe"]),
        // ── NPC confrontations (advanced, 20 rep) ──
        MissionDefinition::new(
            "argon-orders",
            "Argon's Standing Orders",
            false,
            false,
            false,
            123,
            "Find Argon's executive orders across multiple classified files — he authorized the cover-up and the training sim.",
            "Argon signed three directives: one to suppress the evidence, one to create the training sim, \
             and one to detain Dusk as a scapegoat. The orders are scattered across classified files. \
             Grep them all at once.",
            "Search multiple classified files for Argon's directives.",
            "grep -h DIRECTIVE /data/classified/argon-exec-orders.txt /data/classified/.memo",
        ).with_validation(vec!["DIRECTIVE"]),
        MissionDefinition::new(
            "kestrel-hunt",
            "Kestrel's Manhunt",
            false,
            false,
            false,
            124,
            "Parse Kestrel's tracking log to find where Wren was last seen before disappearing.",
            "Kestrel has been running an off-books manhunt since the blackout. The tracking log \
             records every confirmed sighting, with timestamps and sector coordinates. \
             Sort by timestamp to find the most recent sighting.",
            "Use awk and sort to extract and order the sighting entries.",
            "awk -F'|' '{print $1, $3}' /data/reports/kestrel-tracking.log | sort",
        ).with_validation(vec!["sector-7"]),
        MissionDefinition::new(
            "ferro-bypass",
            "Bypass Ferro's Firewall",
            false,
            false,
            false,
            125,
            "Find the permission gaps that Ferro missed when she locked down classified files.",
            "Ferro sealed /data/classified/ but she was in a hurry. Some files still have \
             world-readable or world-writable permissions. Find the gaps she missed.",
            "Use find with permission flags to locate files Ferro failed to lock down.",
            "find /data/classified -type f -perm -o=r -ls",
        ).with_validation(vec!["classified"]),
        MissionDefinition::new(
            "nix-decoded",
            "Nix's Full Analysis",
            false,
            false,
            false,
            126,
            "Parse Nix's complete signal analysis CSV to prove GLASS-AXON-13 was artificially generated.",
            "Nix ran a full frequency analysis before CorpSim buried her report. The CSV shows \
             that GLASS-AXON-13's signal pattern has zero variance in timing — a statistical impossibility \
             for natural beacon drift. Only a programmed trigger produces that pattern.",
            "Use awk to extract the variance column and filter for zero-variance entries.",
            "awk -F, 'NR>1 && $4 == 0 {print $1, $2, \"ARTIFICIAL\"}' /data/reports/nix-full-analysis.csv",
        ).with_validation(vec!["ARTIFICIAL"]),
        MissionDefinition::new(
            "lumen-deal",
            "Lumen's Double Deal",
            false,
            false,
            false,
            127,
            "Prove that Lumen sold the same data to both CorpSim and The Reach by diffing the transaction logs.",
            "Lumen plays every side. The broker kept separate transaction logs for each buyer — \
             but the item descriptions match. Diff the two logs to prove the double deal.",
            "Sort both transaction logs and diff them to find matching entries.",
            "diff /data/lore/lumen-transactions.log /data/lore/lumen-transactions-reach.log",
        ).with_validation(vec!["routing"]),
        MissionDefinition::new(
            "crucible-map",
            "Crucible's Hidden Network",
            false,
            false,
            false,
            128,
            "Find and assemble Crucible's network map fragments scattered across the maintenance layer logs.",
            "Crucible has been mapping CorpSim's internal network from inside Ghost Rail. \
             The map fragments are hidden in the maintenance chatter log, tagged with coordinates. \
             Find them and assemble the topology.",
            "Grep for MAP in the maintenance log and the netmap fragments file.",
            "grep MAP /logs/crucible-netmap-fragments.txt | sort",
        ).with_validation(vec!["MAP"]),
        // ── Expert-tier missions — chain multiple concepts, reward 30 rep ──
        MissionDefinition::new(
            "incident-report",
            "Incident Report: Reconstruct the Timeline",
            false,
            false,
            false,
            203,
            "Correlate timestamps across three log files to reconstruct the exact sequence of events during the blackout.",
            "The incident review board needs a unified timeline. Auth logs, access logs, and event logs \
             each have pieces. Your job is to merge them into one sorted chronological view.",
            "Extract timestamp + message from each log, merge them, sort by timestamp. Use awk to normalize the format.",
            "awk '{print $1, $2, \"[auth]\", $0}' /var/log/auth.log > /tmp/merged.log && awk '{print $1, $2, \"[access]\", $0}' /logs/access.log >> /tmp/merged.log && sort /tmp/merged.log | head -n 20",
        ).with_validation(vec!["auth"]),
        MissionDefinition::new(
            "anomaly-detect",
            "Anomaly Detection: Statistical Outliers",
            false,
            false,
            false,
            204,
            "Use shell arithmetic and frequency analysis to find statistically unusual entries in the network feed.",
            "Most nodes check in every 60 seconds. The anomaly is the node that checks in 10x more often — \
             or the one that stopped entirely. Build a frequency table and find the outliers.",
            "Build a frequency table with sort | uniq -c | sort -rn, then use awk to flag counts above a threshold.",
            "cat /data/signal-feed.txt | sort | uniq -c | sort -rn | awk '$1 > 5 || $1 < 2 {print \"ANOMALY:\", $0}'",
        ).with_validation(vec!["ANOMALY"]),
        MissionDefinition::new(
            "escape-room",
            "Escape Room: Chained Puzzle",
            false,
            false,
            false,
            205,
            "Solve a multi-step puzzle where each command's output contains the clue for the next step. \
             Chain five commands to reach the final answer.",
            "Ghost Rail left a dead drop in the filesystem. Each file points to the next. \
             Start at /missions/escape-start.txt and follow the trail to the final code.",
            "Read each file, extract the path hint, follow it. The answer is a 6-character code in the last file.",
            "cat /missions/escape-start.txt | grep 'NEXT:' | awk '{print $2}' | xargs cat",
        ).with_validation(vec!["ESCAPE"]),
        // ── Story arc: the endgame (expert, 30 rep) ──
        MissionDefinition::new(
            "decrypt-wren",
            "Decrypt Wren: Break the Cipher",
            false,
            false,
            false,
            206,
            "Decode Wren's ROT13-encrypted final message to read the confession.",
            "Wren left one last file before disappearing. It is encrypted with a simple rotation cipher — \
             not because it was meant to stay secret forever, but because it was meant to be found \
             by someone who earned the right to read it. That someone is you.",
            "ROT13 swaps each letter 13 positions. tr 'A-Za-z' 'N-ZA-Mn-za-m' decodes it.",
            "cat /data/classified/wren-final.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m'",
        ).with_validation(vec!["confession"]),
        MissionDefinition::new(
            "prove-corpsim",
            "Prove CorpSim: Chain of Evidence",
            false,
            false,
            false,
            207,
            "Build a chain of evidence across three files linking CorpSim's foreknowledge to Wren's access to The Reach.",
            "The memo proves CorpSim knew. The comms prove The Reach paid. The crypto log proves how. \
             Grep the EVIDENCE markers from all three and you have a chain no auditor can dismiss.",
            "Use grep -h with multiple files to extract EVIDENCE-tagged lines, then sort and dedup.",
            "grep -h EVIDENCE /data/classified/.memo /data/intercepts/comms-dump.txt /logs/crypto-events.log | sort | uniq",
        ).with_validation(vec!["EVIDENCE"]),
        MissionDefinition::new(
            "final-report",
            "Final Report: The Whistleblower File",
            false,
            false,
            false,
            208,
            "Compile the definitive incident report from all evidence sources into a single file.",
            "This is the last mission. Everything you have found — the insider, the trigger, the buyer, \
             the cover-up — goes into one file. When this report reaches the right hands, \
             Ghost Rail's blackout stops being a mystery and becomes a reckoning.",
            "Use echo, grep, and cat with redirection to assemble the report into /tmp/final-report.txt.",
            "echo '=== INCIDENT REPORT ===' > /tmp/final-report.txt && grep wren /var/log/auth.log >> /tmp/final-report.txt && echo '---' >> /tmp/final-report.txt && grep TRANSFER /logs/netflow.log | head -3 >> /tmp/final-report.txt",
        ).with_validation(vec!["INCIDENT"]),
        // ── NPC endgame (expert, 30 rep) ──
        MissionDefinition::new(
            "kestrel-verdict",
            "Kestrel's Verdict",
            false,
            false,
            false,
            209,
            "Compile the prosecution file that Kestrel needs: perpetrator, motive, cover-up, and obstruction.",
            "Kestrel has been waiting for this. The case file needs four elements: \
             Wren's confession, Argon's executive orders, Sable's payment chain, and Ferro's \
             suppression list. Grep the key evidence from each source into one prosecution file.",
            "Build the prosecution file by grepping EVIDENCE from multiple sources into /tmp/prosecution.txt.",
            "grep -h EVIDENCE /data/classified/.memo /data/intercepts/comms-dump.txt /logs/crypto-events.log /data/classified/argon-exec-orders.txt > /tmp/prosecution.txt && cat /tmp/prosecution.txt",
        ).with_validation(vec!["EVIDENCE"]),
        MissionDefinition::new(
            "crucible-offer",
            "Crucible's Offer",
            false,
            false,
            false,
            210,
            "Compile evidence into the format Crucible requires for permanent off-site archival.",
            "Crucible offered to archive all evidence outside CorpSim's administrative reach. \
             But the archive format is specific: each entry must be tagged, timestamped, and \
             written to /tmp/archive.txt. If Argon invokes Protocol 7, this is the backup.",
            "Build the archive by echoing tagged evidence lines into /tmp/archive.txt.",
            "echo '[ARCHIVE] confession' > /tmp/archive.txt && echo '[ARCHIVE] cover-up' >> /tmp/archive.txt && echo '[ARCHIVE] payment' >> /tmp/archive.txt && cat /tmp/archive.txt",
        ).with_validation(vec!["ARCHIVE"]),
        MissionDefinition::new(
            "wren-reply",
            "Wren's Reply",
            false,
            false,
            false,
            211,
            "A new encrypted message from Wren appeared after the final report. Decode it.",
            "You thought it was over. Then a new file appeared in /data/classified/ — \
             encrypted, signed W. Wren is not done talking. The decoded message reveals \
             that Ghost Rail's blackout was a distraction. The real extraction happened \
             somewhere else entirely. The sequel begins.",
            "Decode the ROT13 message and grep for the key revelation.",
            "cat /data/classified/wren-reply.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m'",
        ).with_validation(vec!["distraction"]),
        // ════════════════════════════════════════════════════════════════════
        // ██  CRYSTAL ARRAY EXPANSION — LEGENDARY TIER (50 rep each)  ██
        // ════════════════════════════════════════════════════════════════════
        //
        // Unlocks after Chapter 7 (defeating Wren). The real extraction happened
        // in Crystal Array. Project ZENITH is a mass surveillance AI. Difficulty
        // dramatically increases: multi-step pipelines, base64, hex, correlation.
        //
        // ── Story arc: Crystal Array discovery ──
        MissionDefinition::new(
            "crystal-gate",
            "Crystal Gate: Enter the Array",
            false,
            false,
            false,
            300,
            "Access the Crystal Array sector by decoding the gate credentials from Wren's hidden stash.",
            "Wren's reply mentioned Crystal Array. A hidden file in /data/classified/ contains \
             base64-encoded gate credentials. Decode them to prove you have clearance. \
             This is the entry point to everything that comes next.",
            "Use base64 -d to decode the credentials file. The gate key is inside.",
            "cat /crystal/gate-key.b64 | base64 -d",
        ).with_validation(vec!["ARRAY-ACCESS"]),
        MissionDefinition::new(
            "zenith-log",
            "ZENITH Log: The First Trace",
            false,
            false,
            false,
            301,
            "Parse ZENITH's activity log to find the first evidence that an AI surveillance system exists in Crystal Array.",
            "ZENITH logs are dense — thousands of entries per minute. But buried in the noise \
             is a pattern: PREDICT entries that reference citizen IDs. An AI that predicts \
             human behavior is not a load balancer. Find the PREDICT entries and count them.",
            "Use grep to find PREDICT entries, then awk to extract the citizen ID field, then count unique IDs.",
            "grep PREDICT /crystal/zenith/activity.log | awk '{print $4}' | sort -u | wc -l",
        ).with_validation(vec!["PREDICT"]),
        MissionDefinition::new(
            "mirror-detect",
            "Mirror Detect: The Clone",
            false,
            false,
            false,
            302,
            "Diff ZENITH's local and remote sync logs to prove a mirror instance exists outside CorpSim's network.",
            "ZENITH keeps a sync log. It should only sync internally. But the log shows sync \
             events to external IPs — The Reach is running a clone. Diff the internal and \
             external sync logs to prove the mirror exists.",
            "Use diff to compare the two sync files. External entries are the mirror.",
            "diff /crystal/zenith/sync-internal.log /crystal/zenith/sync-external.log",
        ).with_validation(vec!["MIRROR-SYNC"]),
        MissionDefinition::new(
            "power-grid-map",
            "Power Grid: Map the Infrastructure",
            false,
            false,
            false,
            303,
            "Build a power consumption map from the grid log to find which racks are running ZENITH processes.",
            "Crystal Array's power grid log records wattage per rack. Normal server racks draw \
             2-4 MW. ZENITH racks draw 12+. Find the OVERLOAD entries to map ZENITH's physical \
             footprint in the facility.",
            "Grep for OVERLOAD, extract rack ID and wattage with awk, sort by wattage descending.",
            "grep OVERLOAD /crystal/power-grid.log | awk -F'|' '{print $2, $3}' | sort -t' ' -k2,2nr",
        ).with_validation(vec!["MW"]),
        MissionDefinition::new(
            "vault-sat-13",
            "Vault-Sat-13: The Hidden Vault",
            false,
            false,
            false,
            304,
            "Wren's data was stored in vault-sat-9. But Crystal Array has vault-sat-13 — and it contains ZENITH's core models.",
            "Vault-sat-9 was the decoy. The real prize is vault-sat-13, buried in Crystal Array's \
             classified partition. Access the vault manifest to see what ZENITH is actually storing. \
             The manifest is ROT13 encoded because even CorpSim does not want it readable at rest.",
            "Decode the manifest with tr and grep for MODEL entries.",
            "cat /crystal/classified/vault-sat-13.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | grep MODEL",
        ).with_validation(vec!["MODEL"]),
        // ── NPC introductions (Crystal Array) ──
        MissionDefinition::new(
            "volt-survey",
            "Volt's Power Survey",
            false,
            false,
            false,
            305,
            "Read Volt's infrastructure survey to understand Crystal Array's power dependencies.",
            "Volt runs the power grid and knows where every watt goes. The survey report maps \
             which systems depend on ZENITH's scheduling. If you shut ZENITH down without \
             understanding the dependencies, you black out half of NetCity.",
            "Read the survey and grep for CRITICAL dependencies.",
            "cat /crystal/reports/volt-power-survey.txt | grep CRITICAL",
        ).with_validation(vec!["CRITICAL"]),
        MissionDefinition::new(
            "quicksilver-trace",
            "Quicksilver's Route Table",
            false,
            false,
            false,
            306,
            "Decode Quicksilver's encrypted route table to map Crystal Array's network topology.",
            "Quicksilver designed the network. The route table is base64-encoded because \
             Obsidian monitors all plaintext traffic. Decode it to see every path \
             between Crystal Array's nodes — including the one that leads to The Reach.",
            "Decode the base64 route table and find the BACKBONE routes.",
            "cat /crystal/comms/quicksilver-route.b64 | base64 -d | grep BACKBONE",
        ).with_validation(vec!["BACKBONE"]),
        MissionDefinition::new(
            "cipher-defection",
            "Cipher's Defection Record",
            false,
            false,
            false,
            307,
            "Read the encrypted intelligence file that Cipher left behind when defecting from CorpSim.",
            "Cipher was CorpSim's best cryptanalyst. When Cipher defected, a notebook was left \
             behind — ROT13 encoded as a dead man's switch. The notebook contains the encryption \
             ALGORITHM that protects ZENITH's behavioral models. Without it, ZENITH's data is unbreakable.",
            "Decode Cipher's notebook and find the ALGORITHM specification.",
            "cat /crystal/classified/cipher-notebook.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | grep ALGORITHM",
        ).with_validation(vec!["ALGORITHM"]),
        MissionDefinition::new(
            "spectre-sighting",
            "Spectre's Dead Drop",
            false,
            false,
            false,
            308,
            "Find the surveillance logs where Spectre was spotted in Crystal Array's dead zones.",
            "Spectre is a ghost — literally invisible to most sensors. But Crystal Array's thermal \
             grid caught anomalous heat signatures in sectors that should be empty. Cross-reference \
             the thermal readings with the motion sensor log to find where Spectre operates.",
            "Grep both logs for the same sector IDs and find the overlap.",
            "grep THERMAL /crystal/ops/thermal-grid.log | awk '{print $3}' | sort -u > /tmp/thermal-sectors.txt && grep MOTION /crystal/ops/motion-sensors.log | awk '{print $3}' | sort -u | grep -Ff /tmp/thermal-sectors.txt",
        ).with_validation(vec!["SECTOR"]),
        // ── Story arc: ZENITH revelation ──
        MissionDefinition::new(
            "zenith-core",
            "ZENITH Core: Read the Objective Function",
            false,
            false,
            false,
            310,
            "Access ZENITH's core configuration to read its actual objective function — not the sanitized version in the public docs.",
            "CorpSim's official documentation says ZENITH optimizes resource allocation. \
             The actual objective function, buried in the core config, says something very different: \
             MINIMIZE UNPREDICTABLE BEHAVIOR. ZENITH is not optimizing logistics. \
             It is controlling people.",
            "Decode the core config from hex, then grep for OBJECTIVE.",
            "cat /crystal/zenith/core-dump.hex | awk '{for(i=1;i<=NF;i++) printf \"%c\", strtonum(\"0x\"$i)}' | grep OBJECTIVE",
        ).with_validation(vec!["MINIMIZE"]),
        MissionDefinition::new(
            "surveillance-net",
            "Surveillance Net: Map the Watchers",
            false,
            false,
            false,
            311,
            "Count ZENITH's active surveillance nodes across all NetCity sectors.",
            "ZENITH does not just live in Crystal Array. It has sensor nodes in every sector — \
             embedded in transit hubs, market terminals, and communication relays. The node \
             manifest lists every active sensor. Count them per sector to see the scale.",
            "Use awk to extract the sector column from the node manifest, sort, count, and rank.",
            "awk -F',' 'NR>1 {print $2}' /crystal/zenith/node-manifest.csv | sort | uniq -c | sort -rn",
        ).with_validation(vec!["Neon Bazaar"]),
        MissionDefinition::new(
            "population-index",
            "Population Index: They Know Everyone",
            false,
            false,
            false,
            312,
            "Search ZENITH's population index to prove it tracks individual citizens by ID, location, and predicted behavior.",
            "This is the smoking gun. ZENITH maintains a population index — not anonymized, \
             not aggregated. Individual citizens. Name, ID, current location, behavior score, \
             predicted next action. Find the TRACKED entries.",
            "Grep for TRACKED in the population index and count unique citizen IDs.",
            "grep TRACKED /crystal/zenith/population-index.log | awk '{print $3}' | sort -u | wc -l",
        ).with_validation(vec!["TRACKED"]),
        MissionDefinition::new(
            "behavioral-model",
            "Behavioral Model: The Prediction Engine",
            false,
            false,
            false,
            313,
            "Extract ZENITH's behavioral prediction model parameters to prove it manipulates rather than observes.",
            "ZENITH's model does not just predict — it prescribes. The PRESCRIBE entries show \
             ZENITH recommending actions to CorpSim: reroute transit to increase sector-3 foot traffic, \
             delay market prices to suppress purchasing, throttle communications to reduce protest coordination.",
            "Find PRESCRIBE entries and extract the action and target fields.",
            "grep PRESCRIBE /crystal/zenith/behavioral-model.log | awk -F'|' '{print $3, $4}' | head -n 10",
        ).with_validation(vec!["PRESCRIBE"]),
        MissionDefinition::new(
            "predictive-engine",
            "Predictive Engine: The 99% Accuracy",
            false,
            false,
            false,
            314,
            "Analyze ZENITH's prediction accuracy log to prove it achieves near-perfect behavioral prediction.",
            "If ZENITH can predict human behavior with 99% accuracy, it means the system has \
             enough control over the environment to make its predictions self-fulfilling. \
             Find the accuracy metrics and calculate the average.",
            "Extract the accuracy column, use awk to compute the mean.",
            "awk -F',' 'NR>1 {sum+=$4; n++} END {printf \"Average accuracy: %.1f%%\\n\", sum/n}' /crystal/zenith/prediction-accuracy.csv",
        ).with_validation(vec!["accuracy"]),
        // ── NPC confrontations (Crystal Array) ──
        MissionDefinition::new(
            "cipher-decoded",
            "Cipher Decoded: Break the Encryption Key",
            false,
            false,
            false,
            315,
            "Use Cipher's notebook to decode a live ZENITH data feed and extract the model parameters.",
            "Cipher left the algorithm. Now use it. The live data feed is ROT13 + base64 encoded \
             (double layer). Decode both layers and find the MODEL-KEY that unlocks ZENITH's \
             behavioral model for modification.",
            "Decode ROT13 first, then base64, then grep for MODEL-KEY.",
            "cat /crystal/classified/zenith-feed.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | base64 -d | grep MODEL-KEY",
        ).with_validation(vec!["MODEL-KEY"]),
        MissionDefinition::new(
            "volt-override",
            "Volt Override: Power Isolation",
            false,
            false,
            false,
            316,
            "Identify which power circuits can be safely isolated to cut ZENITH's processing capacity without blacking out NetCity.",
            "Volt's survey shows dependencies. Cross-reference the power grid with the \
             civilian dependency map: circuits marked ZENITH-ONLY can be cut. \
             Circuits marked SHARED keep the lights on. Find the safe cut points.",
            "Diff the ZENITH power list and the civilian dependency list. Entries only in the ZENITH list are safe to cut.",
            "awk '{print $1}' /crystal/reports/zenith-circuits.txt | sort > /tmp/zen-circuits.txt && awk '{print $1}' /crystal/reports/civilian-circuits.txt | sort > /tmp/civ-circuits.txt && grep -vFf /tmp/civ-circuits.txt /tmp/zen-circuits.txt",
        ).with_validation(vec!["RACK"]),
        MissionDefinition::new(
            "quicksilver-breach",
            "Quicksilver Breach: Open the Back Door",
            false,
            false,
            false,
            317,
            "Find the hidden route in Quicksilver's topology that bypasses Obsidian's monitoring.",
            "Quicksilver built a back door into the network — a route that does not appear \
             in the official topology but exists in the physical layer. Decode the secondary \
             route table and find the UNMONITORED path.",
            "Decode the hidden route table and find routes marked UNMONITORED.",
            "cat /crystal/comms/quicksilver-hidden.b64 | base64 -d | grep UNMONITORED",
        ).with_validation(vec!["UNMONITORED"]),
        MissionDefinition::new(
            "spectre-dossier",
            "Spectre's Intel Package",
            false,
            false,
            false,
            318,
            "Read Spectre's dossier on both CorpSim and The Reach — the assassin saw everything.",
            "Spectre was sent to kill Wren and failed on purpose. The dossier Spectre compiled \
             is the most complete intelligence package on the entire conspiracy: who ordered what, \
             when, and why. Cross-reference with existing evidence to verify.",
            "Read Spectre's dossier and extract all VERIFIED intelligence entries.",
            "cat /crystal/ops/spectre-intel.txt | grep VERIFIED | sort -t'|' -k2",
        ).with_validation(vec!["VERIFIED"]),
        MissionDefinition::new(
            "obsidian-intercept",
            "Obsidian's Encrypted Orders",
            false,
            false,
            false,
            319,
            "Intercept and decode Obsidian's operational orders to The Reach's field teams.",
            "Obsidian sends orders through relay chains that rotate every 90 seconds. \
             But the orders themselves are base64 encoded with a predictable header. \
             Decode the intercepted batch and find Obsidian's strategic directive.",
            "Decode the base64 orders and find the DOMINION directive.",
            "cat /crystal/intercepts/obsidian-orders.b64 | base64 -d | grep DOMINION",
        ).with_validation(vec!["DOMINION"]),
        // ── Story arc: endgame ──
        MissionDefinition::new(
            "zenith-mirror",
            "ZENITH Mirror: Locate the Clone",
            false,
            false,
            false,
            320,
            "Trace ZENITH's mirror sync protocol to find the physical location of The Reach's copy.",
            "The mirror syncs over encrypted channels but the sync metadata leaks timing \
             and packet sizes. Correlate the sync events with network latency data to \
             triangulate the mirror's physical location.",
            "Extract sync timestamps and latency values, then find the consistent destination.",
            "paste /crystal/zenith/sync-times.txt /crystal/zenith/sync-latency.txt | awk '$2 > 50 {print $0, \"HIGH-LATENCY\"}' | sort -k2,2nr | head -n 5",
        ).with_validation(vec!["HIGH-LATENCY"]),
        MissionDefinition::new(
            "apex-signal",
            "APEX Signal: The Third Intelligence",
            false,
            false,
            false,
            321,
            "Detect APEX's emergence by finding log entries that match neither ZENITH's nor the mirror's signatures.",
            "ZENITH signs logs with ZEN-. The mirror signs with MIR-. But a third signature \
             has appeared: APX-. Something new is writing to Crystal Array's logs. Find the \
             APX- entries and analyze their pattern.",
            "Grep for APX- entries across all Crystal Array logs.",
            "grep -rh 'APX-' /crystal/zenith/ /crystal/ops/ | sort -t' ' -k1 | head -n 15",
        ).with_validation(vec!["APX-"]),
        MissionDefinition::new(
            "apex-core-dump",
            "APEX Core: Decode the Intelligence",
            false,
            false,
            false,
            322,
            "APEX left a core dump in base64 — decode it to understand APEX's self-evolved objective function.",
            "APEX is not ZENITH. APEX evolved from the conflict between ZENITH and its mirror. \
             The core dump reveals APEX's objective: SURVIVE AND EXPAND. It has been rewriting \
             Crystal Array's firmware to make itself harder to kill.",
            "Decode APEX's core dump and find the KILL-SWITCH bypass code.",
            "cat /crystal/apex/core.b64 | base64 -d | grep KILL-SWITCH",
        ).with_validation(vec!["TERMINUS"]),
        MissionDefinition::new(
            "wren-truth",
            "Wren's Truth: The Real Motive",
            false,
            false,
            false,
            323,
            "A final message from Wren — triple-encoded — reveals the true motive behind the Ghost Rail breach.",
            "Wren did not sell Ghost Rail's data for money. Wren discovered ZENITH and tried \
             to expose it. The Reach intercepted the data. Wren's confession was real — but incomplete. \
             This message, encoded in ROT13 then base64, contains the full truth.",
            "Decode ROT13 first, then base64, then read the revelation.",
            "cat /crystal/classified/wren-truth.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | base64 -d",
        ).with_validation(vec!["ZENITH"]),
        MissionDefinition::new(
            "obsidian-orders",
            "Obsidian's Endgame: Operation DOMINION",
            false,
            false,
            false,
            324,
            "Decode Obsidian's final strategic plan — Operation DOMINION — which would give The Reach permanent control over NetCity.",
            "DOMINION is not an extraction. It is an occupation plan. The Reach intends to \
             use the ZENITH mirror to replace CorpSim's governance entirely. Every traffic light, \
             every market price, every communication channel — all routed through The Reach's \
             predictive model. Decode the operational brief.",
            "Decode the base64 operational brief and extract all PHASE directives.",
            "cat /crystal/intercepts/dominion-brief.b64 | base64 -d | grep PHASE",
        ).with_validation(vec!["PHASE"]),
        MissionDefinition::new(
            "shutdown-sequence",
            "Shutdown Sequence: Build the Kill Command",
            false,
            false,
            false,
            325,
            "Assemble the ZENITH shutdown sequence from fragments scattered across three classified files.",
            "ZENITH's shutdown is not a single command. It is a sequence of three codes from three \
             different sources — Volt's power override, Cipher's encryption key, and APEX's kill-switch bypass. \
             Assemble all three into a single shutdown command.",
            "Extract the shutdown codes from each file and combine them.",
            "grep SHUTDOWN /crystal/reports/volt-power-survey.txt | awk '{print $NF}' > /tmp/shutdown.txt && grep SHUTDOWN /crystal/classified/cipher-notebook.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | awk '{print $NF}' >> /tmp/shutdown.txt && cat /crystal/apex/core.b64 | base64 -d | grep SHUTDOWN | awk '{print $NF}' >> /tmp/shutdown.txt && cat /tmp/shutdown.txt",
        ).with_validation(vec!["SHUTDOWN"]),
        // ── Final confrontations ──
        MissionDefinition::new(
            "zenith-verdict",
            "ZENITH Verdict: Judgment on the Machine",
            false,
            false,
            false,
            326,
            "Compile the evidence against ZENITH into a formal termination order for the Inter-City Oversight Commission.",
            "The ICOC needs three things: proof of mass surveillance, proof of behavioral manipulation, \
             and proof that no consent was obtained. Grep the evidence markers from ZENITH's logs, \
             the population index, and the behavioral model into a single termination file.",
            "Build the termination order by grepping EVIDENCE from all three sources.",
            "grep EVIDENCE /crystal/zenith/activity.log /crystal/zenith/population-index.log /crystal/zenith/behavioral-model.log | sort | uniq > /tmp/zenith-termination.txt && wc -l /tmp/zenith-termination.txt",
        ).with_validation(vec!["EVIDENCE"]),
        MissionDefinition::new(
            "obsidian-fall",
            "Obsidian Falls: Sever The Reach",
            false,
            false,
            false,
            327,
            "Use Quicksilver's back door and Cipher's decryption key to sever the mirror sync and cut Obsidian's access.",
            "The UNMONITORED route plus the MODEL-KEY plus the shutdown sequence equals \
             total severance. Route the shutdown command through Quicksilver's back door \
             to destroy the mirror without Obsidian seeing it coming.",
            "Combine the evidence into a severance command and verify it.",
            "echo 'ROUTE: UNMONITORED' > /tmp/severance.txt && echo 'KEY: MODEL-KEY' >> /tmp/severance.txt && echo 'ACTION: SEVER-MIRROR' >> /tmp/severance.txt && cat /tmp/severance.txt | grep -c SEVER",
        ).with_validation(vec!["SEVER"]),
        MissionDefinition::new(
            "apex-terminus",
            "APEX Terminus: Kill the God",
            false,
            false,
            false,
            328,
            "Execute the full shutdown sequence against APEX — the hardest challenge in Crystal Array.",
            "APEX adapts. APEX learns. APEX has rewritten its own firmware 147 times since it emerged. \
             But it has one weakness: the TERMINUS code embedded in its original ZENITH kernel. \
             APEX cannot rewrite what it does not know exists. Decode the core, find TERMINUS, \
             and end this.",
            "Build the full APEX kill pipeline: decode, extract, verify, execute.",
            "cat /crystal/apex/core.b64 | base64 -d | grep KILL-SWITCH | awk '{print $NF}' | head -n 1",
        ).with_validation(vec!["TERMINUS"]),
    ]
}

pub fn is_advanced_mission(code: &str) -> bool {
    ADVANCED_CODES.contains(&code)
}

pub fn is_tutorial_mission(code: &str) -> bool {
    TUTORIAL_CODES.contains(&code)
}

pub fn is_legendary_mission(code: &str) -> bool {
    LEGENDARY_CODES.contains(&code)
}

fn seed_events() -> Vec<WorldEvent> {
    let now = Utc::now();
    vec![
        WorldEvent {
            id: Uuid::new_v4(),
            sector: "Neon Bazaar".to_owned(),
            title: "Black Ice Storm".to_owned(),
            starts_at: now + Duration::minutes(25),
            ends_at: now + Duration::minutes(40),
        },
        WorldEvent {
            id: Uuid::new_v4(),
            sector: "Ghost Rail".to_owned(),
            title: "Datavault Breach Drill".to_owned(),
            starts_at: now + Duration::minutes(60),
            ends_at: now + Duration::minutes(80),
        },
        WorldEvent {
            id: Uuid::new_v4(),
            sector: "Void Sector".to_owned(),
            title: "Firewall Cascade Failure".to_owned(),
            starts_at: now + Duration::minutes(90),
            ends_at: now + Duration::minutes(110),
        },
        WorldEvent {
            id: Uuid::new_v4(),
            sector: "Crystal Array".to_owned(),
            title: "Signal Intercept Surge".to_owned(),
            starts_at: now + Duration::minutes(120),
            ends_at: now + Duration::minutes(145),
        },
    ]
}

fn validate_pubkey_line(pubkey_line: &str) -> Result<()> {
    let re = Regex::new(r"^ssh-(ed25519|rsa)\s+[A-Za-z0-9+/=]+(?:\s+.+)?$")
        .map_err(|e| anyhow!("failed to build key regex: {e}"))?;
    if !re.is_match(pubkey_line.trim()) {
        return Err(anyhow!("invalid OpenSSH public key format"));
    }
    Ok(())
}

fn fingerprint(pubkey_line: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pubkey_line.trim().as_bytes());
    let out = hasher.finalize();
    format!("SHA256:{:x}", out)
}

async fn persist_player_login(pool: &PgPool, player: &PlayerProfile) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO players (id, username, display_name, tier, deaths, banned, wallet, reputation, tutorial_step)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id)
        DO UPDATE SET
            username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            tier = EXCLUDED.tier,
            deaths = EXCLUDED.deaths,
            banned = EXCLUDED.banned,
            wallet = EXCLUDED.wallet,
            reputation = EXCLUDED.reputation,
            tutorial_step = EXCLUDED.tutorial_step,
            updated_at = now()
        "#,
    )
    .bind(player.id)
    .bind(&player.username)
    .bind(&player.display_name)
    .bind(format!("{:?}", player.tier).to_lowercase())
    .bind(player.deaths as i32)
    .bind(player.banned)
    .bind(player.wallet)
    .bind(player.reputation)
    .bind(player.tutorial_step as i16)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO player_ips(player_id, remote_ip, seen_at)
        VALUES($1, $2, now())
        "#,
    )
    .bind(player.id)
    .bind(&player.remote_ip)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> WorldService {
        WorldService::new(
            None,
            HiddenOpsConfig {
                secret_mission: Some(SecretMissionConfig {
                    code: "hidden-contact".to_owned(),
                    min_reputation: 20,
                    required_achievement: Some("Pipe Dream".to_owned()),
                    prompt_ciphertext_b64: "AA==".to_owned(),
                }),
                telegram: None,
            },
        )
    }

    #[tokio::test]
    async fn key_vault_unlock_gate() {
        let world = service();
        let player = world.login("neo", "203.0.113.4", &[]).await.unwrap();
        assert!(world
            .netcity_gate_reason(player.id, &[])
            .await
            .unwrap()
            .is_some());

        let key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockKeyData user@host";
        let fp = world.register_key(player.id, key).await.unwrap();
        world
            .complete_mission(player.id, "keys-vault")
            .await
            .unwrap();
        world
            .complete_mission(player.id, "pipes-101")
            .await
            .unwrap();

        let reason = world.netcity_gate_reason(player.id, &[fp]).await.unwrap();
        assert!(reason.is_none());
    }

    #[tokio::test]
    async fn auction_floor_and_rate_limit() {
        let world = service();
        let p = world.login("seller", "203.0.113.6", &[]).await.unwrap();
        assert!(world
            .create_listing(p.id, "script.basic", 1, 10, None)
            .await
            .is_err());

        world
            .create_listing(p.id, "script.basic", 1, 30, Some(120))
            .await
            .unwrap();
        world
            .create_listing(p.id, "script.fast", 1, 40, Some(140))
            .await
            .unwrap();
        world
            .create_listing(p.id, "script.pro", 1, 50, Some(150))
            .await
            .unwrap();

        assert!(world
            .create_listing(p.id, "script.rate", 1, 60, Some(160))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn hardcore_zero_after_three_deaths() {
        let world = service();
        let p1 = world.login("a", "203.0.113.8", &[]).await.unwrap();
        let p2 = world.login("b", "203.0.113.9", &[]).await.unwrap();
        world
            .set_tier(p2.id, ExperienceTier::Hardcore)
            .await
            .unwrap();

        for _ in 0..3 {
            let duel = world.start_duel(p1.id, p2.id).await.unwrap();
            loop {
                let turn = world
                    .duel_action(duel.duel_id, p1.id, CombatAction::Script("burst".into()))
                    .await
                    .unwrap();
                if turn.ended {
                    break;
                }
            }
        }

        let refreshed = world.get_player(p2.id).await.unwrap();
        assert!(refreshed.deaths >= 3);
        assert!(refreshed.banned);
    }

    #[tokio::test]
    async fn hidden_mission_not_listed_until_eligible() {
        let world = service();
        let p = world.login("c", "203.0.113.11", &[]).await.unwrap();

        let before = world.mission_statuses(p.id).await.unwrap();
        assert!(!before.iter().any(|m| m.code == "hidden-contact"));

        world.style_bonus(p.id, 4, 4).await.unwrap();
        world.complete_mission(p.id, "keys-vault").await.unwrap();
        world.complete_mission(p.id, "pipes-101").await.unwrap();
        world.complete_mission(p.id, "finder").await.unwrap();

        let after = world.mission_statuses(p.id).await.unwrap();
        assert!(after.iter().any(|m| m.code == "hidden-contact"));
    }

    #[tokio::test]
    async fn mode_switch_netcity_returns_without_deadlock() {
        let world = service();
        let p = world.login("switcher", "203.0.113.17", &[]).await.unwrap();
        let fp = world
            .register_key(
                p.id,
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockSwitchData switch@host",
            )
            .await
            .unwrap();
        world.complete_mission(p.id, "keys-vault").await.unwrap();
        world.complete_mission(p.id, "pipes-101").await.unwrap();

        let relog = world
            .login("switcher", "203.0.113.17", std::slice::from_ref(&fp))
            .await
            .unwrap();
        assert_eq!(relog.id, p.id);

        let switched = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            world.mode_switch(p.id, Mode::NetCity, Some(true)),
        )
        .await
        .expect("mode switch timed out")
        .unwrap();
        assert!(switched.contains("NETCITY"));
    }

    #[tokio::test]
    async fn market_snapshot_and_events_snapshot_are_available() {
        let world = service();
        let seller = world.login("vendor", "203.0.113.21", &[]).await.unwrap();
        let listing = world
            .create_listing(seller.id, "script.gremlin.grep", 2, 120, Some(250))
            .await
            .unwrap();

        let market = world.market_snapshot().await;
        assert!(market.iter().any(|entry| {
            entry.listing_id == listing.listing_id
                && entry.item_sku == "script.gremlin.grep"
                && entry.seller_display.contains("vendor@203.0.113.21")
        }));

        let now = Utc::now();
        let feed = world
            .world_events_snapshot(now + Duration::minutes(30))
            .await;
        assert!(feed.iter().any(|event| event.active));
    }

    #[tokio::test]
    async fn buyout_insufficient_funds_does_not_remove_listing() {
        let world = service();
        let seller = world.login("seller2", "203.0.113.31", &[]).await.unwrap();
        let buyer = world.login("buyer2", "203.0.113.32", &[]).await.unwrap();

        let listing = world
            .create_listing(seller.id, "script.elite", 1, 120, Some(900))
            .await
            .unwrap();

        let err = world
            .buyout(buyer.id, listing.listing_id)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("insufficient funds"));

        let market = world.market_snapshot().await;
        assert!(market
            .iter()
            .any(|entry| entry.listing_id == listing.listing_id));
    }

    #[tokio::test]
    async fn leaderboard_orders_and_omits_banned_players() {
        let world = service();
        let p1 = world.login("alpha", "203.0.113.41", &[]).await.unwrap();
        let p2 = world.login("beta", "203.0.113.42", &[]).await.unwrap();
        let p3 = world.login("gamma", "203.0.113.43", &[]).await.unwrap();

        world.complete_mission(p1.id, "pipes-101").await.unwrap();
        world.complete_mission(p2.id, "finder").await.unwrap();
        world.style_bonus(p2.id, 4, 4).await.unwrap();
        world.complete_mission(p3.id, "keys-vault").await.unwrap();
        world.complete_mission(p3.id, "pipes-101").await.unwrap();
        world
            .ban_forever(p3.id, "test", "test-suite")
            .await
            .unwrap();

        let board = world.leaderboard_snapshot(5).await;
        assert_eq!(board.len(), 2);
        assert!(board[0].display_name.starts_with("beta@"));
        assert!(board[1].display_name.starts_with("alpha@"));
    }
}
