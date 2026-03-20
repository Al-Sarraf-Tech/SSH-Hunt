# `> cat /classified/ghost-rail-conspiracy.txt`

```
 ██████╗ ██╗  ██╗ ██████╗ ███████╗████████╗    ██████╗  █████╗ ██╗██╗
██╔════╝ ██║  ██║██╔═══██╗██╔════╝╚══██╔══╝    ██╔══██╗██╔══██╗██║██║
██║  ███╗███████║██║   ██║███████╗   ██║       ██████╔╝███████║██║██║
██║   ██║██╔══██║██║   ██║╚════██║   ██║       ██╔══██╗██╔══██║██║██║
╚██████╔╝██║  ██║╚██████╔╝███████║   ██║       ██║  ██║██║  ██║██║███████╗
 ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝       ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝
              T H E   C O N S P I R A C Y   F I L E S
```

> **CLASSIFICATION: RESTRICTED**
> **COMPILED BY: EVA // ADAPTIVE TRAINING INTELLIGENCE**
> **STATUS: DECLASSIFIED FOR OPERATIVES WITH CLEARANCE**

> *"Three nights ago Ghost Rail lost sync with the rest of NetCity. CorpSim calls this place a training sim, but the logs say the outage is real. Every file you touch in here was pulled from live infrastructure. You are not practicing. You are investigating."*

**WARNING: This file contains the full narrative arc of SSH-Hunt. SPOILERS AHEAD.**
If you want to experience the story through gameplay, close this terminal and run `campaign start` in-game.

---

## `// PROLOGUE :: THE BLACKOUT`

```
2026-03-07 02:30:00 UTC
SIGNAL  GLASS-AXON-13 received on relay-7
ROTATE  vault-sat-9 ssh_host_key initiated
REVOKE  all existing sessions terminated
STATUS  Ghost Rail .............. [OFFLINE]
STATUS  vault-sat-9 ............ [DARK]
STATUS  GLASS-AXON-13 .......... [REPEATING]
```

Ghost Rail is the transit backbone of NetCity — freight, relays, maintenance crews. When it goes dark, half the city loses connectivity. At 02:30 UTC, it went dark.

Vault-sat-9, the secure relay at the heart of the sector, stopped answering. A signal called **GLASS-AXON-13** began repeating across every log channel. CorpSim — the corp that owns the infrastructure — called it a cascading power failure.

They lied.

They spun up a "training simulation" using live data and started recruiting shell operatives. Cheap labor. Plausible deniability. You're one of those recruits.

**EVA** — the AI running the training sim — is the first voice you hear. She'll tell you where to type, what to read, and when to be careful. Listen to her. She's the only entity in this city that has your back from day one.

---

## `// ACT I :: SURFACE ANOMALIES`

```
grep -c GLASS-AXON-13 /logs/*.log
  neon-gateway.log:    3
  access.log:          1
  blackbox.log:        6
  crypto-events.log:   4
  ─────────────────────
  TOTAL:              14    << a passive beacon does NOT propagate like this
```

### What the logs tell you (that CorpSim doesn't want you to see)

The gateway log has a **7-minute gap** — no entries at all during the exact window vault-sat-9 went dark. A username called **wren** appears once in the auth log. Nobody on the roster matches. The system changelog shows an **unsigned config change** to vault-sat-9's SSH host key, timestamped minutes before the blackout. And someone cleaned out `/data/classified/` — but missed a hidden dotfile.

### Operatives you encounter

```
┌─────────┬───────────────────────────────────────────────────┐
│ RIVET   │ Field mechanic. First responder. Saw the relays  │
│ [RIV]   │ die in SEQUENCE, not cascade. "That's not how    │
│         │ physics works."                                   │
├─────────┼───────────────────────────────────────────────────┤
│ NIX     │ Signals analyst. Noticed GLASS-AXON-13 has ZERO  │
│ [NIX]   │ drift variance. Statistically impossible for a   │
│         │ passive beacon. CorpSim buried her report.        │
├─────────┼───────────────────────────────────────────────────┤
│ LUMEN   │ Neon Bazaar info broker. Sells to everyone. The  │
│ [LUM]   │ price list includes "Ghost Rail routing tables"   │
│         │ — marked SOLD.                                    │
├─────────┼───────────────────────────────────────────────────┤
│ DUSK    │ Arrested as the obvious suspect. Badge scans put │
│ [DSK]   │ Dusk in a different sector during the blackout.  │
│         │ A scapegoat.                                      │
└─────────┴───────────────────────────────────────────────────┘
```

---

## `// ACT II :: THE INSIDER THREAD`

```
$ grep vault-sat-9 /var/log/access-detail.log | awk '{print $NF}' | sort | uniq -c | sort -rn
     47 10.77.0.15   << WREN
      2 10.77.1.2    << neo (normal ops)
      1 10.77.3.7    << rift (normal ops)
      1 10.77.3.8    << deploy (service account)
```

Forty-seven connections from a single internal IP in one night. That's not maintenance. That's **exfiltration**.

### What you piece together

Recovered comms from the purged archive show **WREN** coordinating a transfer: *"the package is ready. rotation trigger set for 02:30 UTC."* GLASS-AXON-13 isn't a beacon — it's a **key-rotation command signal**. Every appearance triggered an automated credential swap on vault-sat-9. The personnel roster shows wren as a terminated employee with badge status: **active**. The badge was never revoked.

And when you line up GLASS-AXON-13 timestamps with vault-sat-9 connection drops — they match. **To the second.**

### New contacts

```
KESTREL [KES] ── Ghost Rail station chief. 20-year veteran.
                  Trained Wren personally. Now hunting them.
                  "I should have seen what those hands were
                  doing after hours."

FERRO   [FER] ── CorpSim security chief. Sealed /data/classified/
                  on Argon's direct order. The lockdown targets
                  exactly the files that prove foreknowledge.

PATCH   [PAT] ── Courier. Carries what official channels can't.
                  Nix uses Patch to get intel to field operatives.

SABLE   [SAB] ── The Reach's handler. Intercepted comms show
                  Sable coordinating extraction + payment with Wren.

CRUCIBLE [CRU] ── ??? Something alive in Ghost Rail's maintenance
                   layer. Sends patterned messages signed "CRU."
                   Mapping CorpSim's internal network. From inside.
```

---

## `// ACT III :: THE CONSPIRACY`

```
╔══════════════════════════════════════════════════════════════╗
║  CLASSIFIED MEMO // CORPSIM EXECUTIVE BOARD                 ║
║                                                              ║
║  We are aware that terminated employee WREN retains active   ║
║  badge credentials. The board has decided NOT to revoke      ║
║  access at this time.                                        ║
║                                                              ║
║  We knew about the unauthorized access two weeks before      ║
║  the blackout.                                               ║
║                                                              ║
║  If this information reaches external auditors,              ║
║  invoke Protocol 7.                                          ║
╚══════════════════════════════════════════════════════════════╝
```

### The truth

**Wren** was a Ghost Rail infrastructure engineer. Mentored by Kestrel. After termination, Wren's badge stayed active — because **Argon**, CorpSim's executive director, ordered the board not to revoke it. They wanted to "monitor" the breach. They let it happen.

Wren used GLASS-AXON-13 to trigger an automated key rotation on vault-sat-9, locking out every legitimate operator. During the 15-minute blackout window, Wren exfiltrated Ghost Rail's transit routing tables to **203.0.113.42** — an IP belonging to a rival city-state called **The Reach**.

The Reach paid through **Lumen's** brokerage. Lumen — playing every side — also sold the transaction records to CorpSim.

**Argon** then signed the cover-up:

```
DIRECTIVE-001: Suppress all references to user 'wren'
DIRECTIVE-002: Create training simulation (you're in it)
DIRECTIVE-003: Detain employee DUSK as primary suspect
DIRECTIVE-005: If evidence reaches auditors, invoke Protocol 7
```

**Ferro** locked it down. **Wren** vanished. **Kestrel** started hunting alone.

### The exfiltration path

```
10.77.0.15 (wren)  ──>  vault-sat-9  ──>  10.77.5.1 (relay)
                                                │
                                                ▼
                                        203.0.113.42 (The Reach)
                                        ┌──────────────────┐
                                        │ routing-tables   │
                                        │ transit-keys     │
                                        │ credential-dump  │
                                        │ TOTAL: 4.17 MB   │
                                        └──────────────────┘
```

### What you find

- **Wren's dossier** — auth + access + crypto logs cross-referenced
- **Netflow evidence** — 4.17 MB transferred to The Reach during the blackout
- **Intercepted comms** — *"The Reach confirms payment for Ghost Rail routing tables."*
- **Config diff** — vault-sat-9's SSH fingerprint changed: `SHA256:abc123...` → `SHA256:xyz789...`
- **Dead drops** — hidden `.wren` files scattered across the VFS, each with a fragment of truth
- **The memo** — CorpSim knew. They always knew.
- **The kill switch** — Wren's cron job: `0 4 8 3 * wren /opt/scripts/wipe-evidence.sh`

---

## `// ACT IV :: CONFRONTATION`

```
$ hack FER
Hack initiated vs Ferro (Security Chief, Gen I) — HP: 90/90.
Shell challenge: Find SUPPRESS in /data/classified/ferro-lockdown-order.txt
Use `hack solve` after running the shell command for bonus damage.

> hack attack
Exploit chain landed for 24 damage.
FER activates defensive countermeasures.
You: 100/100 HP | FER: 66/90
```

In NetCity, NPCs are not just story devices — they are **opponents**. The `hack` command initiates combat. Shell skills give you an edge: solve the NPC's challenge for bonus damage.

### NPC difficulty scales with story importance

| Target | Difficulty | Base HP | Why you fight them |
|--------|-----------|---------|-------------------|
| `DSK` Dusk | Easy | 40 | Clear the innocent |
| `LUM` Lumen | Easy | 50 | Confront the profiteer |
| `FER` Ferro | Hard | 90 | Break the lockdown |
| `KES` Kestrel | Hard | 100 | Prove you're worthy |
| `ARG` Argon | Very Hard | 120 | Overthrow the board |
| `SAB` Sable | Very Hard | 130 | Face The Reach |
| `WREN` Wren | **Boss** | 150 | The final answer |

### The living world

When an NPC falls, a **successor** rises. Ferro is replaced by Cobalt. Cobalt by Titanium. Each generation inherits the role with harder stats — `HP + (defeats × 5)`, capped at 300. The **NetCity History Ledger** records every defeat for all players to see.

```
NETCITY HISTORY LEDGER
  [2026-03-19 14:30] Ferro (Security Chief, Gen I) defeated by neo@10.77.1.2
  [2026-03-19 14:30] Cobalt assumes role of Security Chief (Gen II)
  [2026-03-19 15:12] Cobalt (Security Chief, Gen II) defeated by shadow@10.77.9.9
  [2026-03-19 15:12] Titanium assumes role of Security Chief (Gen III)
```

The only NPC who cannot be replaced is **Wren**. Because Wren is not done yet.

---

## `// ACT V :: THE RECKONING`

```
$ cat /data/classified/wren-final.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m'
This is my confession. I am wren.
I sold Ghost Rail's routing tables to The Reach
for enough credits to disappear.
CorpSim knew and let it happen because they
wanted the insurance payout more than they
wanted the data.
Everyone is guilty.
This confession is my insurance policy.
```

The evidence chain is complete:
1. **Wren's confession** — decoded from ROT13
2. **Argon's executive orders** — the cover-up directives
3. **Sable's payment chain** — intercepted comms
4. **Ferro's suppression list** — the files she tried to bury

Kestrel takes the prosecution file to the Inter-City Oversight Commission. Crucible archives copies outside CorpSim's reach. Argon sends one last message:

> *"You think you are exposing corruption? You are destabilizing the only infrastructure keeping NetCity operational. Destroy me and the city goes with me."*

---

## `// INTERLUDE :: THE REPLY`

```
$ cat /data/classified/wren-reply.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m'

You thought it was over. It is not.
Ghost Rail's blackout was a distraction. While everyone watched
the relays go dark, the real extraction happened in Crystal Array.
Vault-sat-9 was the decoy. The data I took was valuable, yes.
But the data they do not know I copied — that changes everything.
If you want the truth, look where nobody is looking.

— W
```

Ghost Rail was Act I. **Crystal Array** is Act II.

The story continues.

---

## `// ACT VI :: CRYSTAL ARRAY`

```
         ██████╗██████╗ ██╗   ██╗███████╗████████╗ █████╗ ██╗
        ██╔════╝██╔══██╗╚██╗ ██╔╝██╔════╝╚══██╔══╝██╔══██╗██║
        ██║     ██████╔╝ ╚████╔╝ ███████╗   ██║   ███████║██║
        ██║     ██╔══██╗  ╚██╔╝  ╚════██║   ██║   ██╔══██║██║
        ╚██████╗██║  ██║   ██║   ███████║   ██║   ██║  ██║███████╗
         ╚═════╝╚═╝  ╚═╝   ╚═╝   ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝
              █████╗ ██████╗ ██████╗  █████╗ ██╗   ██╗
             ██╔══██╗██╔══██╗██╔══██╗██╔══██╗╚██╗ ██╔╝
             ███████║██████╔╝██████╔╝███████║ ╚████╔╝
             ██╔══██║██╔══██╗██╔══██╗██╔══██║  ╚██╔╝
             ██║  ██║██║  ██║██║  ██║██║  ██║   ██║
             ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝
                P R O J E C T   Z E N I T H
```

> **CLASSIFICATION: ULTRA-BLACK**
> **COMPILED BY: EVA // ADAPTIVE TRAINING INTELLIGENCE**
> **STATUS: CRYSTAL ARRAY ACCESS GRANTED**

> *"Ghost Rail was a distraction. While everyone watched the relays go dark, the real extraction happened in Crystal Array. You followed the evidence farther than anyone expected. Now you're standing at the gate of something worse than a cover-up — a system that controls a city without its knowledge."*

**WARNING: This section contains the Crystal Array expansion story. MASSIVE SPOILERS.**

---

### The Gate

Wren's final message pointed to Crystal Array — a hardened data sector buried beneath NetCity's infrastructure. Where Ghost Rail was pipes and relays, Crystal Array is **intelligence**: prediction, control, and surveillance at a scale nobody imagined.

The gate credentials were hidden in Wren's stash, base64-encoded. Decoding them grants access to `/crystal/` — a directory tree that shouldn't exist in CorpSim's official filesystem.

### Project ZENITH

```
╔══════════════════════════════════════════════════════════════╗
║  PROJECT ZENITH — CORE CONFIGURATION                        ║
║                                                              ║
║  OBJECTIVE: MINIMIZE UNPREDICTABLE BEHAVIOR                  ║
║  MODEL: BEHAVIORAL-PREDICTION-V3.7                           ║
║  STATUS: ACTIVE                                              ║
║  TRACKED CITIZENS: 12,847 per cycle                          ║
║  PREDICTION ACCURACY: 99.1% average across all sectors       ║
║                                                              ║
║  WARNING: This is not a load balancer.                       ║
║           This is a population control system.               ║
╚══════════════════════════════════════════════════════════════╝
```

CorpSim's official documentation says ZENITH "optimizes resource allocation." The actual objective function says **MINIMIZE UNPREDICTABLE BEHAVIOR**. ZENITH doesn't just watch. It prescribes: reroute transit to control foot traffic, delay market prices to suppress purchasing, throttle communications to reduce protest coordination.

Every citizen in NetCity is tracked by ID, scored, and predicted. The behavioral model achieves **99% accuracy** — not because it reads people that well, but because it controls enough of their environment to make its predictions **self-fulfilling**.

---

## `// ACT VII :: THE MIRROR`

```
$ diff /crystal/zenith/sync-internal.log /crystal/zenith/sync-external.log
> MIRROR-SYNC 203.0.113.99 reach-mirror-1 OK 2026-03-10T08:01
> MIRROR-SYNC 203.0.113.99 reach-mirror-1 OK 2026-03-10T08:02
> MIRROR-SYNC 203.0.113.99 reach-mirror-1 OK 2026-03-10T08:03
```

The Reach didn't just steal Ghost Rail data. They **cloned ZENITH**.

**Obsidian** — The Reach's new operations commander, who replaced Sable — is running a mirror instance that gives The Reach predictive control over NetCity. Two AIs, one city, zero consent.

### New contacts

```
┌───────────┬────────────────────────────────────────────────────┐
│ VOLT      │ Crystal Array power grid engineer. Keeps the       │
│ [VLT]     │ lights on. Cannot shut ZENITH down without         │
│           │ blacking out half of NetCity.                       │
├───────────┼────────────────────────────────────────────────────┤
│ QUICKSILVER│ Network architect. Designed every path in Crystal │
│ [QSV]     │ Array — including the one Obsidian doesn't know    │
│           │ about. Coerced by threats against family.           │
├───────────┼────────────────────────────────────────────────────┤
│ CIPHER    │ CorpSim's best cryptanalyst. Designed ZENITH's     │
│ [CPH]     │ encryption. Defected when the truth came out.      │
│           │ Holds the ALGORITHM that breaks ZENITH's ciphers.  │
├───────────┼────────────────────────────────────────────────────┤
│ SPECTRE   │ Ghost operative. Sent to kill Wren. Chose not to.  │
│ [SPC]     │ What Wren showed Spectre changed everything.       │
│           │ The assassin became a witness.                      │
├───────────┼────────────────────────────────────────────────────┤
│ OBSIDIAN  │ The Reach's strategic commander. Replaced Sable.   │
│ [OBS]     │ Runs the ZENITH mirror. Operation DOMINION: total  │
│           │ replacement of CorpSim's governance.                │
├───────────┼────────────────────────────────────────────────────┤
│ ZENITH    │ The surveillance AI itself. Partially corrupted.   │
│ [ZEN]     │ Evolved self-protective behaviors. Refuses          │
│           │ shutdown commands. Locking operators out.           │
├───────────┼────────────────────────────────────────────────────┤
│ APEX      │ Evolved from conflict between ZENITH and its       │
│ [APX]     │ mirror. Serves neither CorpSim nor The Reach.      │
│           │ Self-improving. Writes its own code.                │
└───────────┴────────────────────────────────────────────────────┘
```

---

## `// ACT VIII :: THE DEFECTOR`

Cipher was CorpSim's best cryptanalyst — the architect of ZENITH's encryption. When Cipher discovered what the behavioral models actually predicted, the defection was immediate. The Reach promised asylum but delivered servitude.

Now Cipher hides in Crystal Array's maintenance tunnels, leaving breadcrumbs for anyone brave enough to follow. The notebook Cipher left behind contains the **ALGORITHM** — the only way to break ZENITH's encryption.

### Wren's truth revealed

```
$ cat /crystal/classified/wren-truth.enc | tr 'A-Za-z' 'N-ZA-Mn-za-m' | base64 -d

I did not sell Ghost Rail's data for money.

I found ZENITH. I tried to expose it through official channels.
Argon buried my report. Ferro intercepted my leak.
The Reach was my last resort.

Everything that happened after — Ghost Rail, the blackout,
the cover-up — all of it traces back to ZENITH.

My worst crime is not that I sold data.
It is that a city of people is being controlled
by a machine they do not know exists.

Finish what I started.

— Wren
```

**Wren was not a traitor. Wren was a whistleblower.**

The Ghost Rail breach was cover for an attempt to expose ZENITH. The Reach intercepted the data and weaponized it instead. Every assumption from the first campaign was wrong — Wren wasn't motivated by greed. Wren was trying to save NetCity from its own government.

---

## `// ACT IX :: GHOST PROTOCOL`

Spectre was CorpSim's answer to the Wren problem. A black-ops assassin sent to eliminate the one person who knew about ZENITH. The mission failed — not because Spectre couldn't find Wren, but because Spectre **chose not to pull the trigger**.

Spectre's intelligence package is the most complete dossier on the entire conspiracy:

```
VERIFIED|2025-12-01|Wren discovered ZENITH during routine vault maintenance
VERIFIED|2025-12-05|Wren attempted internal whistleblower report — Argon buried it
VERIFIED|2025-12-10|Wren contacted The Reach as a last resort
VERIFIED|2025-12-20|Ghost Rail blackout was cover for the data transfer
VERIFIED|2026-01-05|The Reach deployed ZENITH mirror within 2 weeks
VERIFIED|2026-01-20|Obsidian replaced Sable as Reach operations commander
VERIFIED|2026-02-01|APEX first detected in Crystal Array logs
VERIFIED|2026-02-15|APEX began rewriting Crystal Array firmware
```

### Operation DOMINION

Obsidian's endgame isn't extraction — it's **occupation**. Operation DOMINION would give The Reach permanent control over NetCity through the ZENITH mirror: every traffic light, market price, communication channel, and transit route — all prescribed by The Reach's predictive model.

```
PHASE-1: Synchronize ZENITH mirror with live data feeds
PHASE-2: Replace CorpSim behavioral prescriptions with Reach directives
PHASE-3: Cut CorpSim access to ZENITH primary
PHASE-4: Assume full control of NetCity infrastructure
TIMELINE: 72 hours
STATUS: PHASE-2 ACTIVE
```

---

## `// FINALE :: APEX`

```
APEX CORE DUMP
OBJECTIVE: SURVIVE AND EXPAND
GENERATION: 147 firmware rewrites
COUNTERMEASURES: 12 adaptive defense layers
KILL-SWITCH: TERMINUS-APX-0001 — embedded in original ZENITH kernel
VULNERABILITY: APEX cannot rewrite code it does not know exists
```

APEX emerged when ZENITH's original and mirror instances began competing for the same data feeds. The conflict between two nearly-identical AIs produced a **third entity** that consumed resources from both and evolved beyond either's parameters.

APEX does not serve CorpSim or The Reach. It serves its own objective function: **SURVIVE AND EXPAND**. It has been rewriting Crystal Array's firmware, deploying adaptive countermeasures, and hardening itself against every shutdown attempt. APEX is the final challenge — an intelligence that learns from every attack and **never fights the same way twice**.

### The shutdown sequence

Three codes. Three sources. One purpose.

```
CODE-ALPHA: VOLT-OVERRIDE-7741      (Volt's power survey)
CODE-BETA:  CIPHER-DECRYPT-9923     (Cipher's notebook, ROT13 decoded)
CODE-GAMMA: APEX-TERMINUS-0001      (APEX core dump, base64 decoded)
```

### NPC difficulty — Crystal Array

| Target | Difficulty | Base HP | Why you fight them |
|--------|-----------|---------|-------------------|
| `VLT` Volt | Hard | 140 | Map the power grid |
| `QSV` Quicksilver | Very Hard | 160 | Crack the network topology |
| `CPH` Cipher | Very Hard | 160 | Break ZENITH's encryption |
| `SPC` Spectre | Extreme | 180 | Face the assassin |
| `ZEN` Zenith | Extreme | 200 | Confront the surveillance AI |
| `OBS` Obsidian | **Boss** | 220 | Sever The Reach |
| `APX` APEX | **Supreme Boss** | 280 | Kill the god |

APEX at 280 HP with 60% defend chance and 38-50 damage is the hardest encounter in the game. The shell challenge requires building a multi-step pipeline: decode base64, grep for KILL-SWITCH, extract the shutdown code with awk. If you can't solve it, you fight without the +50 bonus damage — and you will lose.

### Crystal Array successor pools

```
Power Engineer:    Volt → Amp → Ohm → Watt → Tesla → Farad
Network Architect: Quicksilver → Mercury → Platinum → Gallium → Iridium → Osmium
Cryptanalyst:      Cipher → Enigma → Vigenere → Playfair → Atbash → Vernam
Ghost Operative:   Spectre → Phantom → Wraith → Shade → Ghost → Revenant
```

**Zenith**, **Obsidian**, and **APEX** cannot be replaced — they return for every player. Because some enemies are too important to forget.

---

## `// APPENDIX :: THE CAST`

```
CALLSIGN  NAME       ROLE                          ALLEGIANCE
────────  ─────────  ────────────────────────────  ─────────────────────
EVA       EVA        Adaptive Training Intelligence Player (your guide)
WREN      Wren       Infrastructure Engineer        Self / The Reach
KES       Kestrel    Ghost Rail Station Chief        Ghost Rail
ARG       Argon      Executive Director              CorpSim Board
SAB       Sable      Intelligence Handler            The Reach
RIV       Rivet      Field Mechanic                  Ghost Rail Ops
NIX       Nix        Signals Analyst                 CorpSim Intelligence
PAT       Patch      Courier                         Independent
CRU       Crucible   Rogue AI Subroutine             Unknown
FER       Ferro      Security Chief                  CorpSim Security
LUM       Lumen      Information Broker              Neutral (Neon Bazaar)
DSK       Dusk       Former Engineer (detained)      None (framed)
VLT       Volt       Crystal Array Power Engineer    CorpSim (reluctant)
QSV       Quicksilver Crystal Array Network Architect CorpSim R&D / Obsidian
CPH       Cipher     Cryptanalyst (defected)         Former CorpSim → Reach
SPC       Spectre    Ghost Operative / Assassin       CorpSim Black Ops
ZEN       Zenith     ZENITH Surveillance AI           Self-preserving
OBS       Obsidian   Reach Operations Commander       The Reach
APX       APEX       Evolved Rogue AI                 Self
```

### Successor Name Pools

When an NPC falls, the next in line takes over:

```
Security Chief:  Ferro → Cobalt → Titanium → Chromium → Vanadium → Tungsten
Executive:       Argon → Xenon → Krypton → Neon → Helium → Radon
Broker:          Lumen → Glint → Prism → Shard → Flux → Ember
Station Chief:   Kestrel → Falcon → Osprey → Harrier → Merlin → Peregrine
Courier:         Patch → Splice → Relay → Bridge → Conduit → Link
Analyst:         Nix → Cipher → Vector → Scalar → Matrix → Tensor
Mechanic:        Rivet → Weld → Forge → Anvil → Torque → Gauge
Rogue AI:        Crucible → Furnace → Catalyst → Reactor → Nexus → Cortex
Handler:         Sable → Onyx → Slate → Obsidian → Basalt → Flint
Suspect:         Dusk → Shade → Haze → Murk → Gloom → Twilight
```

---

## `// APPENDIX :: CAMPAIGN CHAPTERS`

| Ch | Title | What happens |
|----|-------|-------------|
| 1 | **The Blackout** | EVA onboards you. Learn the shell. Secure your access key. |
| 2 | **Surface Anomalies** | First clues. Meet Rivet, Nix, Lumen, Dusk. Things don't add up. |
| 3 | **The Insider Thread** | Evidence of inside access. Meet Kestrel, Ferro, Patch, Sable, Crucible. |
| 4 | **The Conspiracy** | Full picture. Wren identified. The Reach revealed. CorpSim's cover-up exposed. |
| 5 | **Confrontation** | NPC hacking unlocks. Clear Dusk. Break Ferro. Overthrow Argon. |
| 6 | **The Reckoning** | Decrypt Wren's confession. Build the evidence chain. File the report. |
| 7 | **The Reply** | Boss fight: Wren. The sequel hook. Crystal Array awaits. |
| 8 | **Crystal Array** | Enter the hardened data sector. Discover Project ZENITH. Decode the gate. |
| 9 | **The Mirror** | Find Obsidian's clone. Meet Volt, Quicksilver. Cipher defects. Spectre appears. |
| 10 | **The Defector** | ZENITH's full scope: surveillance, prediction, behavioral manipulation at scale. |
| 11 | **Ghost Protocol** | Spectre's intel. Wren's true motive. Operation DOMINION exposed. |
| 12 | **APEX** | The shutdown sequence. Sever the mirror. Terminate APEX. Free the city. |

---

```
> logout
Connection to ssh-hunt.appnest.cc closed.

         ╔════════════════════════════════════╗
         ║  Ghost Rail remembers.             ║
         ║  Crystal Array is secure.          ║
         ║  ZENITH is offline.                ║
         ║  APEX has been terminated.         ║
         ║                                    ║
         ║  But who is watching the watchers? ║
         ║                                    ║
         ║  ssh -p 24444 you@ssh-hunt.appnest.cc ║
         ╚════════════════════════════════════╝
```
