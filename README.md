# Anatolia-Sim

[![Version](https://img.shields.io/github/package-json/v/atabeyler/anatolia.bold.sim)](package.json) [![Tests](https://github.com/atabeyler/anatolia.bold.sim/actions/workflows/test.yml/badge.svg)](https://github.com/atabeyler/anatolia.bold.sim/actions/workflows/test.yml) [![License: Proprietary](https://img.shields.io/badge/license-proprietary-red)](LICENSE.txt)

**Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.**

**An agent-based civilization simulator built around a single scientific question:**

> If two DNA-engineered founding individuals are released into a simulated world, can their descendants independently develop consciousness, language, technology, belief systems, and civilization through nothing but genetic inheritance and observational learning?

No individual other than the two founders is ever directly programmed. Every behavior that emerges in subsequent generations must arise from the same two mechanisms that drive real human evolution: **genetic transmission** and **social/observational learning**.

---

## Research Context

The simulation tests whether emergent complexity — language stages, consciousness, cultural norms, religion, law, art, astronomy — can arise from a minimal genetic seed without any scripted shortcuts for non-founder individuals.

- **Motivation:** most agent-based "civilization" simulations script the interesting behavior directly (a leader role assigned by the engine, a belief injected on a timer). That answers "can we render a plausible-looking civilization?", not "does civilization-like complexity actually fall out of genetics and learning alone?" Anatolia-Sim enforces the harder question as a hard constraint (see the Cardinal Rule below) rather than a design guideline.
- **Approach:** two founders carry precisely tuned alleles across 32 gene loci (FOXP2, BDNF, NRXN1, OXTR, …). Every child inherits through Mendelian recombination with ~2 mutations per gamete (~4 per child). Phenotypes — intelligence, curiosity, aggression, language capacity, consciousness potential — flow entirely from the genome, and every subsequent behavior (language stage, belief adoption, social role, migration bias) must trace back to either an inherited allele or something the individual directly observed. 19 concurrent engines (see Architecture below) implement the mechanisms this can plausibly run on — genetics, epigenetics, hormones, psychology, social structure, economy, environment — without ever special-casing a non-founder individual.
- **Validation, deliberately scoped:** where a real-world empirical target exists, the simulation is checked against it rather than tuned by eye — e.g. `mortality.rs`'s per-age-band daily death risk is calibrated against Gurven & Kaplan (2007)'s Siler hazard-model parameters across five real hunter-gatherer populations, with a Monte Carlo harness (`tests/empirical_validation.rs`) that fails CI if the simulated rate drifts outside tolerance of the historical target. Most of the simulation's other mechanisms (consciousness growth, language emergence, belief formation) have no equivalent real-world dataset to validate against — they're internally consistent and grounded in cited research (see Scientific Background below), not independently verified against real civilizational data, because no such dataset of "civilizations built from two genetically-defined founders" exists to validate against.
- **Limitations:** this is a single, non-distributed simulation of population dynamics, not a live-connected experiment — there's no real genetic sequencing, no real anthropological field data feeding in, and no claim that any specific emergent outcome (which belief archetype appears, which technology unlocks first) predicts anything about real human prehistory. The value is in the mechanism (does genetics + observation alone reliably produce language/belief/technology/civilization), not in any single run's specific narrative.

---

## Architecture

The simulation runs 19 concurrent engines per tick (1 tick = 1 simulation day):

| Engine | Purpose |
|---|---|
| **Biology** | Aging, mortality, reproduction, life stages |
| **Genome** | 32-locus Mendelian inheritance, stress-scaled mutation |
| **Epigenetics** | Heritable methylation (BDNF, HPA axis, OXTR, immune priming) |
| **Microbiome** | Gut diversity, infection spread, pathogen immunity |
| **Language** | FOXP2 expression growth, 7-stage emergence, organic vocabulary |
| **Consciousness** | Genetics × language × social context, gated by potential |
| **Psychology** | Wellbeing, stress, theory of mind (0–3), grief, attachment |
| **Hormones** | 49 dynamic hormones across the real HPA/HPT/HPG + digestive/cardiovascular-renal/bone axes, puberty and senescence curves |
| **Agent Behavior / Movement** | Need-driven action selection, land-gated pathfinding, group cohesion |
| **Technology** | Cumulative discovery, 25 techs across 5 tiers (0–4) |
| **Belief** | Proto-beliefs escalating through 6 opaque complexity tiers, named only by the population's own emergent language |
| **Culture** | Meme spread scaled by group consciousness |
| **Art** | 12 art forms, consciousness micro-boost |
| **Architecture** | Settlement building, labor pool, overcrowding events |
| **Law** | Norm emergence, social order, norm violations |
| **Astronomy** | Celestial observations, calendar, eclipse prediction |
| **Social** | Group dynamics, leadership contests, inter-group conflict |
| **Economy** | Foraging, trade, Gini coefficient, astronomy-boosted farming |
| **Environment** | Biome, seasons, weather, natural disasters |

A tick request's own path: `Client → REST/WebSocket → sim-server routes.rs →
runtime.rs tick loop → sim-core's 19 engines in order (see table above) →
save_state (Postgres/SQLite) → derive_stats/serialize_individual → pushed
back over the WebSocket to every watching client.` God Mode interventions
and AI-backed panels (Hypothesis Test, AI Analysis, Documentary) join this
same path at the `sim-server` layer — `god.rs`/`analysis.rs` read or mutate
`SimulationState` directly, then let the next regular tick (or an immediate
`derive_stats` call) propagate the change, rather than running a parallel
state machine of their own.

```
anatolia.bold.sim/
├── rust/
│   ├── sim-core/src/       # Engine crate -- see the table above; genome.rs,
│   │                       #   individual.rs, mortality.rs, reproduction.rs
│   │                       #   live under biology/
│   ├── sim-server/src/     # Axum HTTP/WebSocket server
│   │   ├── main.rs         # Route table, CORS, startup guards
│   │   ├── routes.rs       # Simulation CRUD, checkpoints, reports, exports
│   │   ├── runtime.rs      # Background tick loop, batching/pacing, error recovery
│   │   ├── auth.rs         # Login/register/JWT, local-mode cloud vouching
│   │   ├── db.rs           # Postgres/SQLite backend selection + queries
│   │   ├── ws.rs           # Live-watch WebSocket broadcast
│   │   ├── admin.rs        # Seed-admin, user approval/ban, audit
│   │   ├── god.rs          # God Mode interventions, cross-sim migration
│   │   ├── analysis.rs     # Hypothesis Test / AI Analysis (Gemini + heuristic)
│   │   ├── gemini.rs       # Google Gemini client, model fallback
│   │   ├── email.rs        # Resend-backed transactional email
│   │   └── releases.rs     # Desktop/Android update proxy (GitHub Releases)
│   └── sim-wasm/           # sim-core compiled to wasm32 for WASM-Local mode
├── client/src/
│   ├── components/panels/  # The 31 panels listed below
│   ├── components/simulation/ # SimCreationWizard, live-watch view
│   ├── store/               # Zustand state (simStore.ts)
│   ├── utils/                # API/socket clients, i18n, hormoneGroups.ts
│   └── wasmLocal/            # Browser-only local mode (see AGENTS.md)
├── desktop/                  # Tauri shell (native window + local server launch)
├── render.yaml                # Render deploy blueprint
├── AGENTS.md / CLAUDE.md      # Full technical reference (formulas, loci, routes)
└── CHANGELOG.md               # Notable changes per version
```

This tree covers the directories that matter for understanding the system,
not every file — `AGENTS.md` is the exhaustive technical reference (engine
formulas, the full 32-locus table, every API route) and the source itself
is the final word on any specific behavior.

---

## Key Mechanics

### FOXP2 Expression
Language capacity is not hardcoded. Each individual's `foxp2_expression` starts at 10% of their genetic ceiling at birth and grows through social group interaction. Founders start at 70% (adult-level). Language stages (0–6: pre-linguistic → writing) unlock only when expression thresholds, group size, and generation count are all met.

### Emergent Consciousness
`mind.consciousness` accumulates daily from:
```
Δ = max(potential × 0.001, 0.00015)   ← genetic base rate
  + (lang_stage/6) × 0.0005           ← language bonus
  + 0.0002 (if in group)              ← social ignition bonus
  + (ToM/3) × 0.0003                  ← theory-of-mind bonus
  − stress_level × 0.0003             ← stress penalty
  − (0.3 − hp) × 0.002 (if hp < 0.3) ← injury/illness penalty
```
Hard ceiling: `consciousness_potential × 1.2` — individuals with low genetic potential cannot reach full consciousness regardless of environment.

### Stress-Scaled Mutation
When either parent's `HPA_AXIS` methylation is elevated above its neutral baseline (a chronic-stress signal), `create_gamete()` applies a higher mutation probability to that child's gametes — modeling epigenetically-mediated transgenerational stress response.

### Astronomy-Boosted Farming
A group's accumulated astronomy knowledge (specifically the `seasonal_calendar` discovery) raises `plant_cultivation` yield by letting cultivation be timed to the actual growing season — civilizations that never develop a calendar farm less efficiently once they discover agriculture.

### Quality of Life Index
```
QoL = consciousness×0.3 + (lang_stage/6)×0.2 + health×0.3 + wellbeing×0.2
```

### Dynamic Hormones
Distinct from static genetic traits, `hormones.rs` tracks an actual circulating level per hormone that rises and falls tick by tick — 49 in total (within the ~40-60 range standard endocrinology references cite for the full human set), organized as a real cascade around the HPA (stress: CRH → ACTH → cortisol, norepinephrine → adrenaline, melatonin's real reciprocal coupling), POMC/immune (MSH/endorphin share ACTH's precursor pathway; IL-6/TNF-alpha/interferon are infection-triggered), HPT (metabolic tempo: TSH ↔ thyroid negative feedback, cytokine-suppressed), HPG (reproductive: LH/FSH + DHEA → testosterone/estrogen, progesterone, growth hormone → IGF-1), and bone/calcium (PTH ↔ calcitonin, the real estrogen-bone-protection link) axes, plus a fast/slow metabolic pair (insulin/glucagon, leptin/ghrelin, adiponectin, NPY), a digestive-hormone timescale layer (gastrin through pancreatic polypeptide) over the existing satiation signal, a cardiovascular/renal cascade (renin → angiotensin II → aldosterone, ANP/BNP, EPO) proxied through hydration/HP, and bonding hormones (oxytocin, its more male-leaning counterpart vasopressin, and birth-triggered prolactin). Testosterone/estrogen follow a real puberty ramp and senescence decline (andropause/menopause); an adult founder/migrant starts at their real age-appropriate baseline rather than an infant one. A reproductive-age, non-pregnant female's LH/FSH/estrogen/progesterone additionally ride a ~28-day ovarian cycle (ovulatory LH/FSH/estrogen surge, luteal progesterone rise). Hormones feed back into mortality risk (chronic cortisol, glucagon's fasting-adaptation discount, aldosterone/EPO/PTH-osteoporosis terms), infection severity (interferon), pair-bond strength (dynamic oxytocin/vasopressin), reproduction itself (conception odds and mating urge, both raised by the ovulatory surge/testosterone/estrogen and suppressed by cortisol/prolactin), and a small IGF-1-driven HP recovery boost (stronger before adulthood).

### Seasonal Fertility
Once a community discovers `calendar`, conception odds get a further seasonal nudge (spring highest, winter lowest, ±8% bounded) layered on top of individual FSHR_01-driven fertility — a population with no calendar sees no seasonal pattern at all.

### Genetic Bottleneck / Founder Effect
When a group splits (fission), its small offshoot band breeds largely within itself and its gene pool can drift sharply from the wider population — the Genetic Diversity panel now breaks heterozygosity/allelic variance/inbreeding down per group so this divergence is directly visible, not just inferable.

### Dialect Divergence
Every group independently coins its own word for the same concept from the moment it splits off (word generation is seeded by group ID). The Language panel now shows an actual side-by-side vocabulary comparison across groups for shared concepts, rather than only describing the phenomenon.

### Written Records (Writing Stage)
Once an individual reaches the writing stage, a notable event of the day is committed to their own permanent memory, and any other literate group member can later "read" it from them — knowledge of a past event can now reach someone who never personally witnessed it, as long as both parties are literate.

### Collective Trauma
A disaster or conflict that hits an entire group on the same day leaves a markedly stronger heritable stress-reactivity imprint (via HPA-axis methylation) than one individual's private grief — descendants of disaster survivors carry a measurably stronger inherited stress response.

### Learned Leadership Style
A child whose living parent currently leads their group picks up a small, purely observational bias toward that parent's dominant behavior pattern — nudging (never assigning) which social role the child is more likely to eventually specialize into.

### Cross-Simulation Migration
An explicit, ownership-gated action lets a player carry one individual — full genome, phenotype, epigenome, language and all — from one of their own simulations into another as a new arrival, modeling real inter-population gene flow/migration.

### Kinship-Aware Mate Selection
Nearby eligible mates are no longer picked uniformly at random: an innate, always-on discount (real kin-recognition/Westermarck-style aversion is developmental, not learned) disfavors close relatives as prospective partners, and a group that has culturally learned the `incest_taboo` norm discounts them further still — layered on top of, not replacing, the existing inbreeding-penalty math applied once a pair is actually chosen.

### Genetics-Sensitive Childhood Mortality
Populations here rarely build up a large adult/elder cohort, so childhood deaths dominate a lineage's total mortality — yet which of the two dominant childhood causes (misadventure vs. genetic disease) claimed a child used to be decided by age band alone, ignoring the child's own inherited health/toughness entirely. It now shifts with the child's own genetic quality, so founder genome improvements and generational selection actually matter in the age band that determines most of a population's fate.

### Specific Death Causes, Not a Catch-All
Every death not otherwise explained by disease, starvation, disaster, old age, or a birth complication used to be logged simply as "trauma" — across test runs, roughly half of all deaths fell into that single vague bucket. It's now resolved into the specific circumstance the environment actually supports at the moment of death: exposure (severe cold/heat), a wildlife encounter (scaled to the biome's own predator risk), or the narrower residual of injury.

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Simulation** | Rust (`sim-core` + `sim-server`), agent-based loop |
| **API** | Axum, JWT auth, WebSocket (live stats) |
| **Database** | SQLx async — Postgres for the cloud/multi-user backend, SQLite for local/offline ("Yerel") mode on desktop and Android |
| **Frontend** | React 18, TypeScript, Vite, Tailwind CSS |
| **AI** | Rust heuristics with optional Google Gemini integration (hypothesis testing, civilization analysis, ARIA voice) — falls back to the heuristic path whenever `GEMINI_API_KEY` is unset or a call fails |
| **Desktop** | Tauri shell launching the Rust server |
| **Mobile** | Android app (Capacitor), bundling the same Rust server as a native binary |
| **Deploy** | Render, native Rust binary built via `render.yaml`'s blueprint (no Node backend at runtime) |

---

## Getting Started

### Prerequisites
- Node.js 18+
- Rust toolchain

### Installation

```bash
git clone https://github.com/atabeyler/anatolia.bold.sim.git
cd anatolia.bold.sim

# Client
cd client && npm install
```

### Database Setup

No `DATABASE_URL` is required for local development — the server falls back to a SQLite file created automatically at `rust/sim.db`. Production (Render) sets `DATABASE_URL` to a managed Postgres instance instead; see `render.yaml`.

```bash
cd rust
cargo run -p sim-server
```

Then seed the admin account via API (requires `ADMIN_SEED_TOKEN` from `.env`):

```bash
curl -X POST http://localhost:3001/api/admin/seed-admin \
  -H "x-seed-token: YOUR_ADMIN_SEED_TOKEN" \
  -H "Content-Type: application/json"
```

### Run Locally

```bash
# Terminal 1 — Rust server (port 3001)
cd rust && cargo run -p sim-server

# Terminal 2 — React client (port 5173)
cd client && npm run dev
```

Open `http://localhost:5173`, create a simulation, configure your two founding individuals, and press **Start**.

Desktop and Android builds launch the same Rust server as a bundled local binary — see `desktop/` (Tauri) and `client/android/` (Capacitor).

---

## Environment Variables

Critical — the cloud/Postgres deploy refuses to start (or immediately rejects requests) without these; a plain local dev run tolerates most of them missing by falling back to SQLite and a locally-generated default:

| Variable | Description |
|---|---|
| `DATABASE_URL` | Postgres connection string. On Render (`RENDER_EXTERNAL_URL` set) the server panics at startup if this is missing — it deliberately refuses to fall back to a throwaway SQLite database on the production web deploy. Locally (no `RENDER_EXTERNAL_URL`), an unset `DATABASE_URL` just falls back to a local `rust/sim.db` SQLite file |
| `JWT_SECRET`, `JWT_REFRESH_SECRET` | JWT signing secrets. Required (server panics) only when running as the cloud/Postgres backend — desktop/Android "Yerel" mode has no accounts of its own and vouches tokens through the cloud instead, so it never needs its own secret |
| `ADMIN_SEED_TOKEN`, `ADMIN_USER_CODE`, `ADMIN_PASSWORD`, `ADMIN_EMAIL` | Required together to create the first admin account via `POST /api/admin/seed-admin` (rate-limited to 5 attempts/15 min, constant-time token comparison) — without a seeded admin, nothing in the app can be administered |

Configured, but fails silently if wrong or missing — the app keeps running and looks like it succeeded, so these deserve extra care:

| Variable | What actually happens if it's missing |
|---|---|
| `RESEND_API_KEY` | Registration/approval emails are silently skipped (`email.rs` logs a `tracing::warn!` and returns) — `POST /api/auth/register` still responds `201 Created` / "Awaiting admin approval" regardless of whether the notification email actually went out. If admin approval emails aren't arriving in production, check this first |
| `ADMIN_EMAIL` | Falls back to a hardcoded `info@boldkimya.com.tr` if unset — not a hard failure, but silent if you meant to route admin notifications elsewhere |

Everything else required for normal operation:

| Variable | Description |
|---|---|
| `APP_URL` | The app's externally-reachable URL, used in emailed links. Falls back to Render's own `RENDER_EXTERNAL_URL` if unset, so this rarely needs to be set explicitly on Render itself |
| `CLOUD_API_URL` | The cloud deployment's own URL, used by a desktop/Android "Yerel" (local/SQLite) process to vouch a bearer token against the real cloud account it was issued by. Defaults to `https://anatolia-sim.onrender.com` |
| `SIM_DATA_DIR` | Where the desktop/Android bundled server writes its local SQLite database and checkpoints |

Platform-provided (set by Render itself, or safe to leave at defaults locally):

| Variable | Description |
|---|---|
| `RENDER_EXTERNAL_URL` | Set automatically by Render on a running web service; also doubles as the signal `db.rs` uses to decide "this is the production web deploy, `DATABASE_URL` is mandatory here" |
| `PORT` | The port the server listens on; Render injects this automatically |
| `NODE_ENV` | Affects client build mode; not read by the Rust server itself |

Optional (if unset, the app keeps working on a heuristic/local fallback):

| Variable | What it does |
|---|---|
| `GEMINI_API_KEY` | Enables real Google Gemini calls for Hypothesis Test, AI Analysis, and the Documentary panel's narration. Unset or a failed call both fall back to the same deterministic Rust heuristic path — a missing key is never a hard failure, just a lower-quality narrative |
| `GEMINI_MODEL` | Overrides the default Gemini model id — useful to pin against a dated snapshot rather than silently riding a moving "latest" alias |
| `GITHUB_RELEASES_TOKEN` | Lets the desktop/Android in-app update checkers (`releases.rs`) read this repo's release metadata/assets on your behalf once the repo is private — unnecessary while the repo is public |
| `RAYON_NUM_THREADS`, `TOKIO_WORKER_THREADS` | Manual overrides for the simulation's parallel-tick thread pool and the async runtime's worker count, for constrained hosting environments |

---

## Simulation Controls

- **God Mode** — Trigger earthquakes, floods, epidemics, volcanic eruptions, meteors; toggle quarantine mode to suppress disasters; speak to individuals in their current language stage
- **Hypothesis Test** — State any hypothesis in natural language; evaluated against live simulation data (Gemini when configured, a Rust heuristic otherwise)
- **AI Analysis** — Generate narrative summaries of civilization progress
- **Time Machine** — Save and jump back to any historical checkpoint
- **Fast-Forward** — Skip ahead to a target year without watching every intermediate day
- **Genealogy** — Visualize the founder family tree across generations; select any individual to see an auto-generated biography built from their tracked history
- **Comparative Analysis** — Diff two of your own simulations' key stats side by side (population, language stage, technologies, consciousness, QoL)
- **Cross-Simulation Migration** — Carry one individual, genome and all, from another simulation you own into the current one as a new arrival
- **Reports** — Export a civilization's full history as JSON or a formatted PDF
- **Performance Diagnostics** — Live tick timing, per-engine phase breakdown, DB status
- **Live Watch** — Spectate any currently-running simulation, including ones on another device synced from local ("Yerel") mode
- **Cloud ⇄ Local Transfer** — From a local ("Yerel") or browser dashboard, push any local simulation to your cloud account ("Buluta Yükle"), or pull a cloud simulation down onto the current device ("Yerele İndir"). Each transfer lands as an independent copy — the source keeps running on its own side.
- **Terminate** — Ends a simulation as an in-fiction disaster (archived, not deleted) rather than simply deleting its history; population extinction is also detected automatically and offered as a termination prompt

---

## Panels

| Panel | Shows |
|---|---|
| Population | Individual roster — alive/dead filter, sort, per-person compare, per-individual live hormone breakdown |
| Population Pyramid | Age pyramid, sex ratio, age-group breakdown |
| Biology | Genome/gene expression, individual genetics, life stages |
| Language | Stage, phonology, vocabulary, per-group dialect comparison table |
| Technology | Discovery tree by tier, discoverer stories |
| Belief | Emerged belief systems (named/unnamed), ritual log |
| Culture | Meme emergence, cultural prestige, meme-stage progression |
| Psychology | Wellbeing, stress, consciousness, theory of mind, mood drivers, population-average hormone levels (overall + female/male/child/adult/elderly breakdown) |
| Epigenetics | Methylation levels and inheritance rates (HPA, BDNF, OXTR, …) |
| Genetic Diversity | Heterozygosity, allelic variance, effective population size, inbreeding trend, per-group founder-effect breakdown |
| Genealogy | Family tree from any selected root individual, with an auto-generated biography |
| Social | Groups, leadership changes, conflicts, social event log |
| Economy | Wealth, Gini inequality index, resource levels, trade log |
| Environment | Biome, season, weather metrics, disaster log |
| Architecture | Structures built, tier unlocks, construction log |
| Art | Art/music forms discovered by category, with event log |
| Astronomy | Astronomy knowledge tree, discoveries, discoverer credit |
| Law | Norm/law progression stages, legal event log |
| Microbiome | Pathogen types, sick rate, epidemic history |
| Events | Filterable, searchable log of every simulation event |
| Moments | Curated feed of noteworthy milestone events |
| Legends | Record-holder spotlight (highest consciousness, most children, longest-lived, most reputable, most prolific discoverer) pulled out of an otherwise huge population list |
| Documentary | AI-narrated, scene-by-scene history of the civilization built from its own real tracked events, spanning its full timeline |
| Hypothesis | AI-powered hypothesis testing against live data |
| Analysis | AI chat for open-ended questions about the civilization, plus a two-simulation comparison tool |
| God | Trigger disasters, alter or talk to individuals, quarantine toggle, cross-simulation migration |
| Time Machine | Save and restore simulation checkpoints |
| Report | Export full civilization history as JSON or PDF |
| Performance | Engine tick timing, DB status, connection diagnostics |
| Stats HUD | Draggable floating overlay with live population/food/happiness charts |

---

## How It Works

**Login flow:** user code + password are checked against the cloud/Postgres backend's `users` table (bcrypt-hashed). A successful login issues a short-lived access token (15 minutes) plus a refresh token (30 days). New self-registrations land in `pending` status and email an approval link (7-day-lived, signed with the refresh secret) to `ADMIN_EMAIL` — the account only activates once an admin approves it. Desktop/Android "Yerel" mode has no local account store at all: it forwards the bearer token to the cloud's `/api/auth/me` to vouch for it, caching the result for 60 seconds to avoid a network round trip on every request.

**Tick loop:** each running simulation advances one simulated day at a time through 19 engines in a fixed order (biology → genome → epigenetics → microbiome → language → consciousness → psychology → hormones → agent behavior/movement → technology → belief → culture → art → architecture → law → astronomy → social → economy → environment), batched and paced so the visible simulation speed matches the selected multiplier (see `runtime.rs`'s tick-pacing notes in `AGENTS.md`). A panicking day is caught and logged rather than killing the loop outright; 5 consecutive failures auto-pause the simulation (`GET /:id/diagnostics` surfaces why).

**Simulation creation:** the player names and configures two founders (or accepts God Mode defaults) → `create_founder_for_simulation` seeds their genome/phenotype/epigenome directly (the one place the Cardinal Rule allows direct configuration) → the tick loop takes over, and every descendant's behavior from that point on must trace back to inheritance or observation.

---

## Deployment

The app is deployed on Render via the `render.yaml` blueprint in this repo (native Rust binary, no Node backend at runtime).

| Workflow | Trigger | Purpose |
|---|---|---|
| `.github/workflows/test.yml` (Tests) | Every push to `main`, every PR | Rust core tests + Monte Carlo empirical validation, Rust Clippy (`-D warnings`), sim-server tests, sim-wasm build + native tests + clippy, client tests + typecheck + build |
| `.github/workflows/release.yml` (Desktop Release) | Push to `main` (skips doc-only/Android-only paths), or manual `workflow_dispatch` | Builds and publishes the Windows desktop installer via Tauri; only tags a genuinely new version if `package.json`'s version actually changed since the last release |
| `.github/workflows/android-release.yml` (Android Release) | Push to `main`, or manual `workflow_dispatch` | Builds and publishes the signed Android APK |

Unlike a CI-gated deploy pattern, Render's own auto-deploy (`autoDeployTrigger: commit` in `render.yaml`) rebuilds and redeploys the web service on every push to `main` directly — it does not wait for the Tests workflow to pass first. `render.yaml`'s `buildFilter.ignoredPaths` (root markdown docs, `desktop/**`, `client/android/**`, `.github/**`) skips a rebuild for pushes that touch only those paths, since none of them affect the actual build output. `GET /api/health`'s `version` field reports the exact commit SHA currently live, which is the reliable way to confirm a push has actually reached production rather than assuming from push time alone.

---

## Performance

A single live sample against the production deployment (Render free tier), taken 2026-08-07 — not an average, and free-tier cold-start/neighbor-noise variance is real. Treat as a rough order of magnitude, not an SLA.

| Endpoint | Observed | Notes |
|---|---|---|
| `GET /api/health` | ~0.9s | No DB/auth involved; the only endpoint measurable here without an authenticated session |

Most other endpoints (simulation state, population lists, tick WebSocket) require an authenticated session and weren't measured for this table — see `PerformancePanel` in a running simulation for live per-engine tick timing and DB round-trip cost instead, which is the more meaningful number for this app (a fixed request latency matters far less here than sustained tick throughput at high simulation speed).

---

## Security Notes

- Passwords are bcrypt-hashed in the cloud backend's `users` table, never stored in plain text
- Access tokens are short-lived (15 minutes); refresh tokens last 30 days
- The registration-approval link mailed to the admin is a signed JWT (reusing the refresh secret, tagged with a distinct `purpose` claim) valid for 7 days — possession of the link is the authorization, there's no separate login step to approve a pending user
- Desktop/Android "Yerel" mode holds no account secrets of its own: every bearer token is vouched for by calling the real cloud's `/api/auth/me`, so a compromised local device never exposes the JWT signing secret itself
- Login, registration, and admin-seeding are all rate-limited (login: 10/15 min per user code; registration: 20/15 min globally, since an attacker controls every field including the user code; seed-admin: 5/15 min globally, with constant-time token comparison against `ADMIN_SEED_TOKEN`)
- `genetic_boost` (God Mode) only ever applies to founders — see the Cardinal Rule; this is enforced by a source-scanning test (`tests/cardinal_rule_source_scan.rs`), not just a convention
- All admin notifications go to `ADMIN_EMAIL` — see the Environment Variables table above for what happens if `RESEND_API_KEY` isn't configured

---

## Scientific Background

The project draws on:

- **FOXP2** — The "language gene"; expression drives communication stage progression
- **Theory of Mind** — Numeric 0–3 scale; gates social complexity and belief formation
- **Epigenetic inheritance** — BDNF, HPA axis, and OXTR methylation are heritable across 2 generations with configurable heritability coefficients
- **Inbreeding coefficient** — Computed from shared grandparents; elevated inbreeding reduces phenotype fitness
- **Cultural transmission fidelity** — Meme spread rate scales with group consciousness, modeling the observation that more cognitively complex societies transmit culture more faithfully
- **X-linked recessive traits** — A hemophilia-like clotting-factor locus is modeled the same way real X-linked conditions are: sons express their single maternal allele directly, daughters need two low copies to show reduced clotting
- **Founder effect / genetic bottleneck** — A small offshoot band's gene pool can drift sharply from the wider population once it breeds largely within itself, mirroring real island/colonization genetics
- **Transgenerational epigenetic inheritance of trauma** — A collective (group-wide) disaster leaves a stronger heritable stress-reactivity imprint than one individual's private stress, echoing real research on inherited trauma responses

---

## Project Notes

> No individual other than the two founders may be given any behavior except through genetic inheritance and observational learning. This constraint is the entire point of the experiment.

---

## Roadmap

Shipped:

- ✅ 49-hormone dynamic endocrine system across the real HPA/HPT/HPG + digestive/cardiovascular-renal/bone axes
- ✅ Mechanistic wound-based predator/injury/exposure/wildlife-encounter deaths, replacing a flat probability roll
- ✅ Mortality rates recalibrated against real hunter-gatherer empirical data (Gurven & Kaplan 2007), validated by a Monte Carlo CI test
- ✅ WASM-Local browser-only mode with genuine multithreading (`wasm-bindgen-rayon`), no server round trip required
- ✅ Cross-simulation migration, kinship-aware mate selection, per-group dialect divergence, written records & inter-individual "reading"

Under consideration (real gaps found during development, not a promised timeline):

- ⬜ Movement system's panic-return-to-land (`_lastLandX/Y`) and food-memory direction (`_goodFoodAngle`) are flagged in-code as not yet implemented
- ⬜ An iOS port — `rust/sim-wasm` exists specifically as groundwork for this (iOS doesn't allow a persistent bundled subprocess the way Android does), but isn't wired into any shipped build yet
- ⬜ Admin login has no 2FA — once seeded, the highest-privilege account authenticates with the same single-factor password check as any other approved user
- ⬜ A local/WASM-mode reverse listing (browsing local simulations from a pure Cloud session) isn't offered, since Cloud has no network path to an arbitrary device's local data

---

## FAQ

**Why did my Hypothesis Test / AI Analysis come back as a short, formulaic summary instead of a rich narrative?** `GEMINI_API_KEY` is either unset or the Gemini call failed — both fall back to the same deterministic Rust heuristic path rather than erroring out. Not a bug; a missing key is an explicitly supported configuration, not a broken one.

**A fix I know is merged doesn't seem to be present in the desktop/Android app — is the deploy broken?** Probably not the web deploy. Unlike the plain web app (always serves the latest built JS from Render), a native desktop/Android build bundles the client at CI build time — it only picks up a fix once that device's app is actually updated/reinstalled from the corresponding `Desktop Release`/`Android Release` workflow run. Check `GET /api/health`'s `version` field against the fixing commit's SHA to confirm the *server* is current before assuming anything is wrong.

**A user says they never received the registration-approval email — what do I check?** First, is `RESEND_API_KEY` actually set? If it's missing, the server silently reports registration success without sending anything (see Environment Variables). If it is set, check `ADMIN_EMAIL` and the Resend dashboard for delivery failures.

**Can two closely related individuals (e.g. full siblings) still have children?** Yes — kinship-aware mate selection discounts related candidates heavily (and further still once a group has culturally learned the `incest_taboo` norm) but never reduces the odds to exactly zero, and the existing inbreeding-coefficient fertility penalty still applies afterward. A related pair remains a possible, just disfavored, pairing.

**Why did a simulation stop advancing with no visible error?** Check `GET /:id/diagnostics` — the tick loop auto-pauses a simulation after 5 consecutive per-day panics and logs each one to a 20-entry circular buffer there.

---

## Troubleshooting

| Symptom | Likely cause | Check |
|---|---|---|
| Simulation stuck at status `paused` with no user action | 5 consecutive tick panics auto-paused it | `GET /:id/diagnostics`'s `error_log` |
| Registration approval email never arrives, but the API reports success | `RESEND_API_KEY` not configured | See the FAQ entry above |
| Deploy doesn't seem to reflect a recent push | Either the push hasn't propagated yet, or it only touched a path `render.yaml`'s `buildFilter.ignoredPaths` skips | Compare `GET /api/health`'s `version` field (the live commit SHA) against the pushed commit |
| `cargo clippy --workspace --all-targets -- -D warnings` fails in CI | A lint (often `doc_lazy_continuation` on a multi-line `///` comment) introduced by a recent change | Run the same command locally before pushing — CI runs it verbatim |
| A native (desktop/Android) build doesn't show a fix that's live on the web | Native builds bundle the client at CI build time, not at runtime | See the FAQ entry above; reinstall from the latest release workflow run |
| Live watch screen looks frozen on Android's "Bulut" (cloud) mode specifically | The device's WebSocket host resolution predates the `isNativeAndroidApp()`/`CLOUD_API_URL` fix (see `AGENTS.md`'s Live watch connection notes) | Confirm the installed app build is recent enough to include that fix |

---

## Citation

If you use or reference Anatolia-Sim's methodology (the genetics-and-
observation-only emergence constraint, the mortality/hormone calibration
approach, or any of the individual engine designs), please cite:

```
@software{anatoliasim2026,
  title        = {Anatolia-Sim: An Agent-Based Civilization Simulator
                   Testing Emergent Complexity from Genetic Inheritance
                   and Observational Learning Alone},
  author       = {{Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.}},
  year         = {2026},
  url          = {https://github.com/atabeyler/anatolia.bold.sim},
  note         = {Two genetically-defined founders and their descendants,
                   modeled with no directly-programmed non-founder behavior
                   -- language, consciousness, belief, technology, and
                   culture emerge solely from Mendelian inheritance and
                   observational learning}
}
```

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for the history of notable changes.

---

## License

Proprietary — see [LICENSE.txt](LICENSE.txt). All rights reserved; this source is not licensed for copying, modification, or redistribution without the Company's prior written consent.

© Bold Askeri Teknoloji ve Savunma Sanayi A.Ş. · All Rights Reserved
