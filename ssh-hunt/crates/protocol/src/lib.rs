#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Mode {
    Training,
    NetCity,
    Redline,
}

impl Mode {
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Training => "SOLO TRAINING SIM",
            Self::NetCity => "MULTIPLAYER NETCITY MMO",
            Self::Redline => "REDLINE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    pub player_id: Uuid,
    pub username: String,
    pub remote_ip: String,
    pub display_name: String,
    pub key_fingerprints: Vec<String>,
    pub observed_key_fingerprints: Vec<String>,
}

impl PlayerIdentity {
    pub fn new(player_id: Uuid, username: String, remote_ip: String) -> Self {
        let display_name = format!("{username}@{remote_ip}");
        Self {
            player_id,
            username,
            remote_ip,
            display_name,
            key_fingerprints: Vec::new(),
            observed_key_fingerprints: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: Uuid,
    pub identity: PlayerIdentity,
    pub node: String,
    pub cwd: String,
    pub mode: Mode,
    pub flash_enabled: bool,
    pub last_exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub raw: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissionState {
    Locked,
    Available,
    Active,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionStatus {
    pub code: String,
    pub title: String,
    pub state: MissionState,
    pub progress: u8,
    pub required: bool,
    pub starter: bool,
    pub summary: String,
    pub suggested_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: Uuid,
    pub sku: String,
    pub qty: u32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionListing {
    pub listing_id: Uuid,
    pub seller_id: Uuid,
    pub item_sku: String,
    pub qty: u32,
    pub start_price: i64,
    pub buyout_price: Option<i64>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub channel: String,
    pub sender_display: String,
    pub body: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub id: Uuid,
    pub sector: String,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub id: Uuid,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub read: bool,
    pub received_at: DateTime<Utc>,
}

/// Player's combat stance — determines whether other players can challenge them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CombatStance {
    #[default]
    Pve,
    Pvp,
}

/// A record of an NPC defeat or succession event in the NetCity history ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub event: String,
    pub npc_name: String,
    pub npc_role: String,
    pub generation: u32,
    pub defeated_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRunResult {
    pub output: String,
    pub exit_code: i32,
    pub consumed_ops: u64,
    pub elapsed_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn fixed_uuid() -> Uuid {
        Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap()
    }

    fn roundtrip<T>(value: &T) -> T
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let json = serde_json::to_string(value).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn mode_label_covers_all_variants() {
        assert_eq!(Mode::Training.as_label(), "SOLO TRAINING SIM");
        assert_eq!(Mode::NetCity.as_label(), "MULTIPLAYER NETCITY MMO");
        assert_eq!(Mode::Redline.as_label(), "REDLINE");
    }

    #[test]
    fn mode_serde_roundtrip() {
        for variant in [Mode::Training, Mode::NetCity, Mode::Redline] {
            let back: Mode = roundtrip(&variant);
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn mission_state_serde_roundtrip() {
        for variant in [
            MissionState::Locked,
            MissionState::Available,
            MissionState::Active,
            MissionState::Completed,
        ] {
            let back: MissionState = roundtrip(&variant);
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn combat_stance_default_is_pve() {
        assert_eq!(CombatStance::default(), CombatStance::Pve);
    }

    #[test]
    fn combat_stance_serde_roundtrip() {
        for variant in [CombatStance::Pve, CombatStance::Pvp] {
            let back: CombatStance = roundtrip(&variant);
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn player_identity_constructor_builds_display_name() {
        let id = fixed_uuid();
        let identity = PlayerIdentity::new(id, "alice".to_string(), "10.0.0.1".to_string());
        assert_eq!(identity.display_name, "alice@10.0.0.1");
        assert!(identity.key_fingerprints.is_empty());
        assert!(identity.observed_key_fingerprints.is_empty());
        assert_eq!(identity.player_id, id);
    }

    #[test]
    fn player_identity_serde_roundtrip() {
        let original = PlayerIdentity::new(fixed_uuid(), "bob".to_string(), "1.2.3.4".to_string());
        let back: PlayerIdentity = roundtrip(&original);
        assert_eq!(back.player_id, original.player_id);
        assert_eq!(back.username, original.username);
        assert_eq!(back.remote_ip, original.remote_ip);
        assert_eq!(back.display_name, original.display_name);
    }

    #[test]
    fn session_context_serde_roundtrip() {
        let ctx = SessionContext {
            session_id: fixed_uuid(),
            identity: PlayerIdentity::new(fixed_uuid(), "u".into(), "10.0.0.1".into()),
            node: "edge-01".into(),
            cwd: "/home/u".into(),
            mode: Mode::NetCity,
            flash_enabled: true,
            last_exit_code: 0,
        };
        let back: SessionContext = roundtrip(&ctx);
        assert_eq!(back.node, ctx.node);
        assert_eq!(back.cwd, ctx.cwd);
        assert_eq!(back.mode, ctx.mode);
        assert_eq!(back.flash_enabled, ctx.flash_enabled);
    }

    #[test]
    fn command_request_serde_roundtrip() {
        let req = CommandRequest {
            raw: "ls -la".into(),
            timestamp: fixed_ts(),
        };
        let back: CommandRequest = roundtrip(&req);
        assert_eq!(back.raw, req.raw);
        assert_eq!(back.timestamp, req.timestamp);
    }

    #[test]
    fn command_result_serde_roundtrip() {
        let res = CommandResult {
            stdout: "ok".into(),
            stderr: String::new(),
            exit_code: 0,
        };
        let back: CommandResult = roundtrip(&res);
        assert_eq!(back.stdout, res.stdout);
        assert_eq!(back.exit_code, res.exit_code);
    }

    #[test]
    fn mission_status_serde_roundtrip() {
        let m = MissionStatus {
            code: "intro-01".into(),
            title: "Intro".into(),
            state: MissionState::Active,
            progress: 42,
            required: true,
            starter: false,
            summary: "first mission".into(),
            suggested_command: "help".into(),
        };
        let back: MissionStatus = roundtrip(&m);
        assert_eq!(back.code, m.code);
        assert_eq!(back.state, m.state);
        assert_eq!(back.progress, m.progress);
    }

    #[test]
    fn inventory_item_serde_roundtrip() {
        let item = InventoryItem {
            item_id: fixed_uuid(),
            sku: "credits-100".into(),
            qty: 7,
            metadata: serde_json::json!({"rarity":"common"}),
        };
        let back: InventoryItem = roundtrip(&item);
        assert_eq!(back.item_id, item.item_id);
        assert_eq!(back.qty, item.qty);
        assert_eq!(back.metadata, item.metadata);
    }

    #[test]
    fn auction_listing_serde_roundtrip() {
        let listing = AuctionListing {
            listing_id: fixed_uuid(),
            seller_id: fixed_uuid(),
            item_sku: "blade".into(),
            qty: 1,
            start_price: 100,
            buyout_price: Some(500),
            expires_at: fixed_ts(),
        };
        let back: AuctionListing = roundtrip(&listing);
        assert_eq!(back.listing_id, listing.listing_id);
        assert_eq!(back.start_price, listing.start_price);
        assert_eq!(back.buyout_price, listing.buyout_price);
    }

    #[test]
    fn chat_message_serde_roundtrip() {
        let msg = ChatMessage {
            id: fixed_uuid(),
            channel: "#general".into(),
            sender_display: "alice@10.0.0.1".into(),
            body: "hello".into(),
            sent_at: fixed_ts(),
        };
        let back: ChatMessage = roundtrip(&msg);
        assert_eq!(back.channel, msg.channel);
        assert_eq!(back.body, msg.body);
    }

    #[test]
    fn world_event_serde_roundtrip() {
        let ev = WorldEvent {
            id: fixed_uuid(),
            sector: "neon-bazaar".into(),
            title: "Black Ice Storm".into(),
            starts_at: fixed_ts(),
            ends_at: fixed_ts(),
        };
        let back: WorldEvent = roundtrip(&ev);
        assert_eq!(back.sector, ev.sector);
        assert_eq!(back.title, ev.title);
    }

    #[test]
    fn mail_message_serde_roundtrip() {
        let mail = MailMessage {
            id: fixed_uuid(),
            from: "ops@netcity".into(),
            subject: "wake up".into(),
            body: "the city needs you".into(),
            read: false,
            received_at: fixed_ts(),
        };
        let back: MailMessage = roundtrip(&mail);
        assert_eq!(back.subject, mail.subject);
        assert_eq!(back.read, mail.read);
    }

    #[test]
    fn history_entry_serde_roundtrip() {
        let h = HistoryEntry {
            event: "defeat".into(),
            npc_name: "Vex".into(),
            npc_role: "fixer".into(),
            generation: 3,
            defeated_by: "alice".into(),
            timestamp: fixed_ts(),
        };
        let back: HistoryEntry = roundtrip(&h);
        assert_eq!(back.npc_name, h.npc_name);
        assert_eq!(back.generation, h.generation);
    }

    #[test]
    fn script_run_result_serde_roundtrip() {
        let r = ScriptRunResult {
            output: "result".into(),
            exit_code: 0,
            consumed_ops: 1234,
            elapsed_ms: 42,
        };
        let back: ScriptRunResult = roundtrip(&r);
        assert_eq!(back.output, r.output);
        assert_eq!(back.consumed_ops, r.consumed_ops);
        assert_eq!(back.elapsed_ms, r.elapsed_ms);
    }
}
