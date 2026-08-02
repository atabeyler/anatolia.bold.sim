# Anatolia-Sim

**An agent-based civilization simulator built around a single scientific question:**

> If two DNA-engineered founding individuals are released into a simulated world, can their descendants independently develop consciousness, language, technology, belief systems, and civilization through nothing but genetic inheritance and observational learning?

No individual other than the two founders is ever directly programmed. Every behavior that emerges in subsequent generations must arise from the same two mechanisms that drive real human evolution: **genetic transmission** and **social/observational learning**.

---

## Core Hypothesis

The simulation tests whether emergent complexity — language stages, consciousness, cultural norms, religion, law, art, astronomy — can arise from a minimal genetic seed without any scripted shortcuts for non-founder individuals.

Founders carry precisely tuned alleles across 32 gene loci (FOXP2, BDNF, NRXN1, OXTR, …). Every child inherits through Mendelian recombination with ~2 mutations per gamete (~4 per child). Phenotypes — intelligence, curiosity, aggression, language capacity, consciousness potential — flow entirely from the genome.

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
| **Hormones** | 20 dynamic hormones across the real HPA/HPT/HPG endocrine axes, puberty and senescence curves |
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
Distinct from static genetic traits, `hormones.rs` tracks an actual circulating level per hormone that rises and falls tick by tick — 20 in total, organized as a real cascade around the HPA (stress: ACTH → cortisol, norepinephrine → adrenaline), HPT (metabolic tempo: TSH ↔ thyroid negative feedback), and HPG (reproductive: LH + DHEA → testosterone/estrogen, progesterone, growth hormone) axes, plus a fast/slow metabolic pair (insulin/glucagon, leptin/ghrelin) and bonding hormones (oxytocin, its more male-leaning counterpart vasopressin, and birth-triggered prolactin). Testosterone/estrogen follow a real puberty ramp and senescence decline (andropause/menopause); feeds back (small, bounded terms) into mortality risk (chronic cortisol, glucagon's fasting-adaptation discount) and pair-bond strength (dynamic oxytocin/vasopressin).

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
| Population | Individual roster — alive/dead filter, sort, per-person compare |
| Population Pyramid | Age pyramid, sex ratio, age-group breakdown |
| Biology | Genome/gene expression, individual genetics, life stages |
| Language | Stage, phonology, vocabulary, per-group dialect comparison table |
| Technology | Discovery tree by tier, discoverer stories |
| Belief | Emerged belief systems (named/unnamed), ritual log |
| Culture | Meme emergence, cultural prestige, meme-stage progression |
| Psychology | Wellbeing, stress, consciousness, theory of mind, mood drivers, population-average hormone levels |
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

## License

MIT
