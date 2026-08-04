use std::collections::{HashMap, HashSet};

// wasm-bindgen-rayon (sim-wasm's initThreadPool, worker.ts) gives wasm32 a
// real Web-Worker-backed rayon thread pool via SharedArrayBuffer + COOP/COEP
// headers -- but this macro used to unconditionally fall back to a plain
// sequential iterator on that target regardless, dating from before that
// infrastructure existed and never updated once it landed. sim-core's own
// par_iter_mut calls were therefore *always* single-threaded in the browser,
// silently, even with the thread pool successfully initialized -- every bit
// of that SharedArrayBuffer/COOP-COEP/initThreadPool machinery provided zero
// speedup since the day it was added. rayon itself builds fine for
// wasm32-unknown-unknown (it's a plain, non-target-gated dependency; only
// getrandom/uuid need the wasm32-specific "js" feature) and correctly falls
// back to sequential execution on its own if the thread pool was never
// initialized (no cross-origin isolation, initThreadPool failed, etc.), so
// there's no need for this crate to make that decision itself -- one real
// parallel macro for every target.
use rayon::prelude::*;
use serde_json::{json, Value};
use web_time::Instant;

use crate::{
    agent, architecture, art, astronomy, belief, biology,
    client_view::{individual_display_name, pascal_to_snake},
    consciousness, culture, economy, environment, epigenetics, hormones, language, law,
    microbiome, milestones, psychology, social, spatial::SpatialGrid, technology, PhaseTimings,
    SimulationState, TickReport,
};

macro_rules! maybe_par_iter_mut {
    ($e:expr) => {
        $e.par_iter_mut()
    };
}

const GROUP_RADIUS: f64 = 3.0;
const NEARBY_RADIUS: f64 = 2.0;
const MATE_SEARCH_RADIUS: f64 = 4.0;
const WITNESS_RADIUS: f64 = 2.0;
const MAX_EVENTS: usize = 1000;
const DAILY_STEP: f64 = 0.015;
// Rough flat lon/lat-degree-to-km conversion (~111km/degree at the equator).
// Migration distance is a narrative/statistical metric, not a navigational
// one, so this doesn't need latitude-corrected longitude scaling.
const KM_PER_DEGREE: f64 = 111.0;
// A logged "migration" is a genuine band relocation, not day-to-day foraging
// jitter -- both a minimum distance and a minimum time since the last logged
// migration are required so a band oscillating near the threshold doesn't
// spam the event/report log.
const MIGRATION_MIN_KM: f64 = 3.0;
const MIGRATION_MIN_INTERVAL_DAYS: i32 = 10;
// A young/tightly-clustered band can have every member within NEARBY_RADIUS/
// MATE_SEARCH_RADIUS of every other member, so the SpatialGrid alone doesn't
// bound candidate counts -- without this cap, observation-based learning
// clones the entire nearby group for every individual in it (O(group_size^2)
// full Individual clones, the dominant per-tick cost once a group grows past
// a few dozen members). Capping to a bounded local sample is also more
// realistic: nobody actually surveys their whole band every day.
const MAX_NEARBY_SAMPLE: usize = 10;
// `SpatialGrid::candidates_within` is coarse (whole 3x3/5x5 cell block, not an
// exact-radius query) and over-inclusive by construction -- callers filter the
// result down by exact distance/eligibility before applying MAX_NEARBY_SAMPLE.
// Without a cap on the *raw* candidate list itself, that per-candidate
// filtering cost scales with however many people share a cell block, which
// keeps climbing for as long as a settlement keeps attracting more residents
// (nothing ever splits or thins one out) -- an unbounded O(local_density) scan
// per individual, and therefore O(local_density^2) per tick for that
// settlement, even though only MAX_NEARBY_SAMPLE candidates are ever kept.
// Capping the raw list first bounds that scan the same way MAX_NEARBY_SAMPLE
// already bounds the kept/cloned sample.
const MAX_CANDIDATE_SCAN: usize = 50;
// Juvenile dependency: below this age, movement blends toward a living
// parent's position (fading to zero pull as they approach it). Matches the
// is_adult cutoff already used for mate-eligibility elsewhere, so "child"
// and "adult" mean the same thing across the engine.
const JUVENILE_MAX_AGE_YEARS: f64 = 13.0;

fn as_string_set(items: &[String]) -> HashSet<String> {
    items.iter().cloned().collect()
}

fn from_string_set(set: HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    v
}

/// A cheap stand-in for a full `Individual`, carrying only the fields the
/// observation-learning pass below actually reads (id/position for the
/// spatial filter, known_techs/vocabulary for the learning itself). Everyone
/// alive gets cloned into this pass's snapshot *and* into every eligible
/// learner's capped nearby-sample every tick, so cloning the real struct there
/// -- genome, epigenome, inventory, social/psychology relationship maps, all
/// the free-form `extra`/`memory` JSON -- meant paying for a full-population
/// copy of state that pass has no use for. Everything left at `..Default::default()`
/// is an empty String/Vec/HashMap/HashSet, so cloning a stub is O(known_techs
/// + vocabulary size) instead of O(everything an Individual ever accumulates).
fn observation_stub(ind: &crate::state::Individual) -> crate::state::Individual {
    crate::state::Individual {
        id: ind.id.clone(),
        x: ind.x,
        y: ind.y,
        alive: true,
        known_techs: ind.known_techs.clone(),
        language: crate::types::Language { vocabulary: ind.language.vocabulary.clone(), writing: ind.language.writing, ..Default::default() },
        // Only cloned for literate individuals (rare, late-game) -- everyone
        // else's memory is dropped here the same way genome/epigenome/
        // inventory already are, to avoid paying for it on every alive
        // individual every tick. See language::read_written_records.
        memory: if ind.language.writing { ind.memory.clone() } else { Value::Null },
        ..Default::default()
    }
}

/// Everything `apply_movement` and `nearest_fertile_opposite_sex` actually
/// read off the pre-move snapshot: position/id (centroid + nearest-neighbor
/// lookups), sex + birth_day (is_fertile's own only inputs, via get_age), and
/// group_id (band centroid grouping). A full `.cloned()` of every alive
/// individual here (as this used to be) pays for genome/epigenome/inventory/
/// skills/beliefs/language.vocabulary/memory on every single tick even though
/// movement never looks at any of them -- and unlike observation_stub's
/// candidates (bounded per learner by MAX_CANDIDATE_SCAN/MAX_NEARBY_SAMPLE),
/// *every* alive individual gets fully cloned into this one, so the wasted
/// clone cost scales directly with living population and with how much
/// larger those unused maps have grown over each individual's lifetime.
fn movement_stub(ind: &crate::state::Individual) -> crate::state::Individual {
    crate::state::Individual {
        id: ind.id.clone(),
        x: ind.x,
        y: ind.y,
        alive: true,
        sex: ind.sex.clone(),
        birth_day: ind.birth_day,
        group_id: ind.group_id.clone(),
        ..Default::default()
    }
}

fn distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

// Long enough that any next-day-only reaction to a death (witnessing, grief)
// has already fired (see apply_death_witnessing's `yesterday = current_day -
// 1`) before this runs, with slack to spare.
pub const DEAD_FIELD_STRIP_GRACE_DAYS: i32 = 7;

/// Mirrors what the pre-Rust-migration JS engine did immediately on death
/// (see server/src/engines/simulationLoop.js's `load()`, which nulled
/// genome/epigenome/phenotype/mind/social/memory/psychology/inventory/
/// beliefs/language/skills/health/known_techs down to just `_name`/`_intel`
/// for display): once dead, an individual is never touched again by any pass
/// that mutates individuals (every one already skips `is_dead`/`!alive`), so
/// keeping their full genome/epigenome/inventory/skills/beliefs/memory/
/// social/psychology/known_techs forever only bloats state_json and the
/// individuals table with data nothing will ever read again -- and unlike
/// total-alive, total-ever-born only ever grows for the life of a
/// simulation.
///
/// Narrower than the JS precedent: phenotype/mind/language/health are left
/// alone, since the client's population comparison view (PopulationPanel's
/// CompareModal) still displays those for a deceased individual, and nothing
/// else needs the rest. Runs once per individual -- `individual.genome`
/// being already empty is this pass's own signal that a dead individual has
/// already been stripped, so there's no separate marker to maintain.
fn strip_dead_individual_if_due(individual: &mut crate::state::Individual, current_day: i32) {
    if !individual.is_dead || individual.genome.is_empty() {
        return;
    }
    let Some(death_day) = individual.death_day else { return };
    if current_day - death_day < DEAD_FIELD_STRIP_GRACE_DAYS {
        return;
    }
    individual.genome.clear();
    individual.epigenome.clear();
    individual.inventory.clear();
    individual.skills.clear();
    individual.beliefs.clear();
    individual.memory = Value::Null;
    individual.social = Default::default();
    individual.psychology = Default::default();
    individual.known_techs.clear();
}

/// H-16 parity: generation is a pedigree-depth field, not inferred from age, and
/// only *alive* individuals count -- otherwise a dead high-generation lineage
/// would permanently inflate language-stage eligibility for everyone still alive.
fn max_alive_generation(individuals: &[crate::state::Individual]) -> i32 {
    individuals.iter().filter(|i| i.alive && !i.is_dead).filter_map(|i| i.generation).max().unwrap_or(0)
}

/// Parent/child (either direction) or shared-bio-parent siblings/half-siblings.
fn are_kin(a: &crate::state::Individual, b: &crate::state::Individual) -> bool {
    let is_parent_child = a.parent_1_id.as_deref() == Some(b.id.as_str())
        || a.parent_2_id.as_deref() == Some(b.id.as_str())
        || b.parent_1_id.as_deref() == Some(a.id.as_str())
        || b.parent_2_id.as_deref() == Some(a.id.as_str());
    if is_parent_child {
        return true;
    }
    let a_parents = [a.parent_1_id.as_deref(), a.parent_2_id.as_deref()];
    let b_parents = [b.parent_1_id.as_deref(), b.parent_2_id.as_deref()];
    a_parents.iter().flatten().any(|ap| b_parents.iter().flatten().any(|bp| ap == bp))
}

/// Maps a death cause to the `_fears` key a witness should update -- water
/// fear is tracked in its own dedicated `_waterFear` field (see
/// `apply_death_witnessing`) rather than under `_fears`.
fn cause_to_fear_key(cause: &str) -> &'static str {
    match cause {
        "predator" | "wildlife_encounter" => "predator",
        "conflict" => "conflict",
        "infection" => "infection",
        // AGENTS.md documents `scarcity` as one of the six _fears keys, but
        // nothing ever actually wrote it -- a starved or dehydrated death's
        // witnesses fell into the unspecific "general" bucket instead of the
        // resource-anxiety response that's the obvious fit here.
        "starvation" | "dehydration" => "scarcity",
        // Disaster-type deaths (environment::process_disaster's own
        // dead_ids) already bump the whole population's "disaster" fear
        // directly; mapping their cause here too keeps a *witnessed*
        // disaster death consistent with that instead of falling to
        // "general".
        "earthquake" | "flood" | "wildfire" | "blizzard_disaster" | "drought_event" => "disaster",
        _ => "general",
    }
}

/// Cardinal-rule-compliant "death witnessing": yesterday's deaths raise the
/// relevant fear in nearby survivors, weighted by proximity and doubled for
/// kin (matching AGENTS.md's documented 0.7/0.4 proximity weights) -- this is
/// a uniform rule applied identically to every individual from objective
/// facts (position, genealogy, cause of death), not scripted per-individual
/// behavior. Returns per-witness `death_of_kin` events for the psychology
/// pass immediately following this in `advance_one_day`, which already
/// implements the grief/trauma response but previously never received any
/// events to react to.
fn apply_death_witnessing(state: &mut SimulationState, current_day: i32) -> Vec<Value> {
    let yesterday = current_day - 1;
    let deceased: Vec<crate::state::Individual> = state
        .individuals
        .iter()
        .filter(|i| i.is_dead && i.death_day == Some(yesterday))
        .cloned()
        .collect();
    if deceased.is_empty() {
        return Vec::new();
    }
    // Bounded via SpatialGrid like every other proximity pass in this file
    // (movement, mating, observation-learning, trade/disease). This used to
    // be a raw nested scan (every survivor x every death), which was cheap
    // on an ordinary day but turned a mass-casualty event -- a disaster with
    // mortality_factor near 1.0, or an epidemic peak, both modeled elsewhere
    // in this file -- into an O(survivors x deaths) pass on exactly the tick
    // that's already the most expensive one, since it wasn't gated by
    // MAX_CANDIDATE_SCAN the way every other spatial query here is.
    let survivor_indices: Vec<usize> =
        state.individuals.iter().enumerate().filter(|(_, ind)| ind.alive && !ind.is_dead).map(|(i, _)| i).collect();
    let survivor_positions: Vec<(f64, f64)> = survivor_indices.iter().map(|&i| (state.individuals[i].x, state.individuals[i].y)).collect();
    let survivor_grid = SpatialGrid::build(&survivor_positions, WITNESS_RADIUS);

    let mut kin_events = Vec::new();
    for dead in &deceased {
        for local_idx in survivor_grid.candidates_within(dead.x, dead.y, WITNESS_RADIUS, MAX_CANDIDATE_SCAN) {
            let survivor = &mut state.individuals[survivor_indices[local_idx]];
            let dist = distance(survivor.x, survivor.y, dead.x, dead.y);
            if dist >= WITNESS_RADIUS {
                continue;
            }
            let proximity = (1.0 - dist / WITNESS_RADIUS).clamp(0.0, 1.0);
            let kin = are_kin(survivor, dead);
            let weight = (if kin { 0.7 } else { 0.4 }) * proximity;
            if weight <= 0.0 {
                continue;
            }
            let cause = dead.extra.get("death_cause").and_then(Value::as_str).unwrap_or("");
            if cause == "drowning" {
                let current = survivor.extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
                survivor.extra.insert("_waterFear".to_string(), json!((current + weight).min(1.0)));
            } else {
                let key = cause_to_fear_key(cause);
                let mut fears = survivor.extra.get("_fears").cloned().unwrap_or_else(|| json!({}));
                let current = fears.get(key).and_then(Value::as_f64).unwrap_or(0.0);
                if let Some(obj) = fears.as_object_mut() {
                    obj.insert(key.to_string(), json!((current + weight).min(1.0)));
                }
                survivor.extra.insert("_fears".to_string(), fears);
            }
            if kin {
                kin_events.push(json!({ "type": "death_of_kin", "individual_id": survivor.id }));
            }
        }
    }
    kin_events
}

/// Union-find over currently ungrouped, living individuals so nearby people cluster
/// into bands. Real settlements/groups then persist via `group_id` once formed.
fn form_groups(state: &mut SimulationState, current_day: i32) {
    let ungrouped_idx: Vec<usize> = state
        .individuals
        .iter()
        .enumerate()
        .filter(|(_, ind)| ind.alive && !ind.is_dead && ind.group_id.is_none())
        .map(|(i, _)| i)
        .collect();

    let group_centroids: Vec<(String, f64, f64)> = state
        .groups
        .iter()
        .filter_map(|g| {
            let id = g.get("id")?.as_str()?.to_string();
            let x = g.get("territory")?.get("x")?.as_f64()?;
            let y = g.get("territory")?.get("y")?.as_f64()?;
            Some((id, x, y))
        })
        .collect();

    let mut still_ungrouped = Vec::new();
    for idx in ungrouped_idx {
        let (ix, iy) = (state.individuals[idx].x, state.individuals[idx].y);
        let joined = group_centroids
            .iter()
            .find(|(_, gx, gy)| distance(ix, iy, *gx, *gy) < GROUP_RADIUS)
            .map(|(id, ..)| id.clone());
        if let Some(gid) = joined {
            state.individuals[idx].group_id = Some(gid.clone());
            if let Some(group) = state.groups.iter_mut().find(|g| g.get("id").and_then(Value::as_str) == Some(gid.as_str())) {
                push_member(group, &state.individuals[idx].id);
            }
        } else {
            still_ungrouped.push(idx);
        }
    }

    // Cluster the rest via union-find, using a spatial grid so only nearby
    // pairs are ever distance-checked instead of every pair in the population.
    let n = still_ungrouped.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }
    let local_positions: Vec<(f64, f64)> = still_ungrouped.iter().map(|&i| (state.individuals[i].x, state.individuals[i].y)).collect();
    let local_grid = SpatialGrid::build(&local_positions, GROUP_RADIUS);
    for a in 0..n {
        let (ax, ay) = local_positions[a];
        // Same MAX_CANDIDATE_SCAN cap this file already applies to mating
        // search and observation-based learning, for the identical reason:
        // candidates_within is coarse and over-inclusive, so an increasingly
        // dense settlement (nothing here ever splits or thins one out) would
        // otherwise turn this union-find pass into an unbounded
        // O(local_density) scan per individual -- O(local_density^2) for
        // that settlement's tick -- even though a person only ever really
        // clusters with a bounded local sample of neighbors, not everyone
        // sharing their cell block.
        for b in local_grid.candidates_within(ax, ay, GROUP_RADIUS, MAX_CANDIDATE_SCAN) {
            if b <= a {
                continue;
            }
            let (bx, by) = local_positions[b];
            if distance(ax, ay, bx, by) < GROUP_RADIUS {
                let ra = find(&mut parent, a);
                let rb = find(&mut parent, b);
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, &member_idx) in still_ungrouped.iter().enumerate() {
        let root = find(&mut parent, a);
        clusters.entry(root).or_default().push(member_idx);
    }
    for members in clusters.values() {
        if members.len() < 2 {
            continue;
        }
        let cx = members.iter().map(|&i| state.individuals[i].x).sum::<f64>() / members.len() as f64;
        let cy = members.iter().map(|&i| state.individuals[i].y).sum::<f64>() / members.len() as f64;
        let group_id = format!("group_{}_{}", current_day, uuid::Uuid::new_v4());
        let member_ids: Vec<Value> = members.iter().map(|&i| json!(state.individuals[i].id)).collect();
        let group = json!({
            "id": group_id,
            "member_ids": member_ids,
            "leader_id": Value::Null,
            "founded_day": current_day,
            "territory": { "x": cx, "y": cy },
            "internal_tension": 0.3,
            "norms": [],
            "culture": [],
        });
        state.groups.push(group);
        for &i in members {
            state.individuals[i].group_id = Some(group_id.clone());
        }
    }

    // Drop empty groups (all members dead/departed).
    state.groups.retain(|g| {
        g.get("member_ids")
            .and_then(Value::as_array)
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    });
}

// Mutates member_ids in place instead of cloning the whole array out, pushing,
// and reinserting it -- a group that keeps attracting new members one at a
// time over its history (the realistic long-term case for any settlement that
// doesn't stop growing) used to pay O(current_size) for the clone alone on
// every single join, which is O(group_size^2) total work over the group's
// lifetime just from this bookkeeping.
//
// The duplicate check that used to guard the push is gone in release builds
// (kept as a debug_assert instead): this function has exactly one call site,
// form_groups's "join an existing group" loop, which only ever calls it for
// an `idx` drawn from `ungrouped_idx` -- built by filtering on
// `ind.group_id.is_none()` -- and immediately sets that individual's
// `group_id` afterward. An id can therefore never already be present when
// this runs; the check was an O(current_size) scan per call (so still
// O(group_size^2) over a group's lifetime) guarding a case that can't
// happen.
fn push_member(group: &mut Value, id: &str) {
    if let Some(members) = group.get_mut("member_ids").and_then(Value::as_array_mut) {
        debug_assert!(!members.iter().any(|v| v.as_str() == Some(id)), "push_member: {id} is already a member");
        members.push(json!(id));
    }
}

/// Removes `dead_ids` from every group's `member_ids`, mirroring what
/// `environment::process_disaster` already does for disaster deaths. See
/// advance_one_day's step 9b for why every other death path needs this too.
fn prune_dead_from_groups(groups: &mut [Value], dead_ids: &[String]) {
    if dead_ids.is_empty() {
        return;
    }
    for group in groups.iter_mut() {
        if let Some(members) = group.get_mut("member_ids").and_then(Value::as_array_mut) {
            members.retain(|v| !dead_ids.iter().any(|id| v.as_str() == Some(id.as_str())));
        }
    }
}

/// Drops any settlement whose owning group no longer exists in `groups` --
/// mirrors form_groups's own `state.groups.retain(...)` (which already drops
/// a group once its member_ids goes empty) one level down. Group ids are
/// freshly-generated UUIDs per formation (see form_groups), never reused, so
/// once a group_id is gone it is gone forever -- nothing a later tick does
/// can "bring back" a settlement whose group has already been dropped, which
/// is what makes this safe to prune eagerly rather than needing a grace
/// window like the individuals/genealogy bounding does.
///
/// Before this, a settlement outlived its group indefinitely: every one ever
/// founded stayed in `state.settlements` forever, and every single tick
/// re-processed all of them (`process_architecture_tick`,
/// `check_settlement_overcrowding`, the `has_settlement` scan below) even
/// for groups that died out decades of simulated time ago -- a per-tick cost
/// that only ever grew with a simulation's total history of group
/// formation/collapse, not its current population, the same shape of bug
/// the genealogy-index fix (db.rs) addressed on the DB-load side.
fn prune_orphaned_settlements(settlements: &mut Vec<Value>, groups: &[Value]) {
    let live_group_ids: std::collections::HashSet<&str> = groups.iter().filter_map(|g| g.get("id").and_then(Value::as_str)).collect();
    settlements.retain(|s| s.get("group_id").and_then(Value::as_str).is_some_and(|gid| live_group_ids.contains(gid)));
}

fn nearest_fertile_opposite_sex<'a>(
    ind: &crate::state::Individual,
    snapshot: &'a [crate::state::Individual],
    grid: &SpatialGrid,
    current_day: i32,
) -> Option<&'a crate::state::Individual> {
    grid.candidates_within(ind.x, ind.y, MATE_SEARCH_RADIUS, MAX_CANDIDATE_SCAN)
        .into_iter()
        .filter_map(|idx| snapshot.get(idx))
        .filter(|other| {
            other.id != ind.id
                && other.alive
                && !other.is_dead
                && other.sex != ind.sex
                && biology::individual::is_fertile(other, current_day)
        })
        .take(MAX_NEARBY_SAMPLE)
        .min_by(|a, b| {
            distance(ind.x, ind.y, a.x, a.y)
                .partial_cmp(&distance(ind.x, ind.y, b.x, b.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Movement follows the priority order documented for the simulation: survival
/// stress (already expressed as the chosen action) -> band cohesion -> a
/// persisted wander heading -> mating drive. A move is only committed if the
/// destination is still on land, so bands never drift out into open ocean.
/// `navigation_bonus` is the civilization's astronomy-derived `navigation`
/// bonus (see `astronomy::get_astronomy_bonus`) -- star/lunar tracking lets
/// explorers cover more ground per day without getting turned around, so it
/// only scales the "explore" action's step, not band cohesion/mating/flee.
fn apply_movement(state: &mut SimulationState, snapshot: &[crate::state::Individual], grid: &SpatialGrid, current_day: i32, navigation_bonus: f64, weather_move_mult: f64) {
    let mut group_centroids: HashMap<String, (f64, f64, usize)> = HashMap::new();
    for ind in snapshot.iter().filter(|i| i.alive && !i.is_dead) {
        if let Some(gid) = &ind.group_id {
            let entry = group_centroids.entry(gid.clone()).or_insert((0.0, 0.0, 0));
            entry.0 += ind.x;
            entry.1 += ind.y;
            entry.2 += 1;
        }
    }
    let group_centroids: HashMap<String, (f64, f64)> = group_centroids
        .into_iter()
        .map(|(gid, (sx, sy, n))| (gid, (sx / n as f64, sy / n as f64)))
        .collect();
    // Living parents' pre-move positions, for the juvenile-dependency pull
    // below -- `snapshot` is already alive-only (see advance_one_day).
    let positions_by_id: HashMap<&str, (f64, f64)> = snapshot.iter().map(|i| (i.id.as_str(), (i.x, i.y))).collect();

    for individual in state.individuals.iter_mut() {
        if !individual.alive || individual.is_dead {
            continue;
        }
        let action = individual.extra.get("_currentAction").and_then(Value::as_str).unwrap_or("explore").to_string();
        if action == "rest" || action == "craft" {
            continue;
        }
        let persisted_angle = individual.extra.get("_moveAngle").and_then(Value::as_f64).unwrap_or_else(|| rand::random::<f64>() * std::f64::consts::TAU);

        let (mut angle, step) = match action.as_str() {
            "flee" => (persisted_angle + std::f64::consts::PI + (rand::random::<f64>() - 0.5), DAILY_STEP * 4.0),
            "explore" => (persisted_angle + (rand::random::<f64>() - 0.5) * 1.2, DAILY_STEP * 2.5 * (1.0 + navigation_bonus)),
            "mate" => {
                if let Some(target) = nearest_fertile_opposite_sex(individual, snapshot, grid, current_day) {
                    ((target.y - individual.y).atan2(target.x - individual.x), DAILY_STEP)
                } else if let Some((cx, cy)) = individual.group_id.as_ref().and_then(|g| group_centroids.get(g)) {
                    ((cy - individual.y).atan2(cx - individual.x), DAILY_STEP)
                } else {
                    (persisted_angle, DAILY_STEP * 0.5)
                }
            }
            "socialize" => {
                if let Some((cx, cy)) = individual.group_id.as_ref().and_then(|g| group_centroids.get(g)) {
                    ((cy - individual.y).atan2(cx - individual.x), DAILY_STEP)
                } else {
                    (persisted_angle, DAILY_STEP * 0.5)
                }
            }
            _ => (persisted_angle + (rand::random::<f64>() - 0.5) * 0.3, DAILY_STEP), // forage/hunt/drink/seek_warmth
        };
        // weather_move_mult previously existed as a computed field nothing
        // anywhere ever read -- a blizzard/storm/heavy_rain slowing everyone
        // down had no actual effect on movement.
        let step = step * weather_move_mult;
        angle = angle.rem_euclid(std::f64::consts::TAU);

        let mut dx = angle.cos() * step;
        let mut dy = angle.sin() * step;

        // Juvenile dependency: a child too young to fend for itself stays
        // close to a living parent (falling back to the group/band if
        // orphaned) instead of wandering off on its own action-driven path.
        // The pull fades linearly to zero by JUVENILE_MAX_AGE_YEARS, driven
        // purely by age and genealogy -- an objective rule applied uniformly
        // to every non-founder, not scripted behavior for specific individuals.
        let age_years = individual.age_days.unwrap_or(0) as f64 / 365.0;
        if age_years < JUVENILE_MAX_AGE_YEARS {
            let caregiver = [individual.parent_1_id.as_deref(), individual.parent_2_id.as_deref()]
                .into_iter()
                .flatten()
                .find_map(|pid| positions_by_id.get(pid))
                .copied()
                .or_else(|| individual.group_id.as_ref().and_then(|g| group_centroids.get(g)).copied());
            if let Some((cx, cy)) = caregiver {
                let pull = 1.0 - (age_years / JUVENILE_MAX_AGE_YEARS);
                let to_caregiver = (cy - individual.y).atan2(cx - individual.x);
                dx = dx * (1.0 - pull) + to_caregiver.cos() * DAILY_STEP * pull;
                dy = dy * (1.0 - pull) + to_caregiver.sin() * DAILY_STEP * pull;
            }
        }

        let candidate_x = individual.x + dx;
        let candidate_y = individual.y + dy;
        if environment::is_on_land(candidate_y, candidate_x) {
            individual.x = candidate_x;
            individual.y = candidate_y;
        }
        // Coastline: don't strand the band, just don't advance this tick.
        individual.extra.insert("_moveAngle".to_string(), json!(angle));
    }
}

/// Logs a `migration` event whenever the living band's average position has
/// drifted far enough from its last-logged baseline, so the report's
/// "Migration History" section (previously always hardcoded empty, see
/// `routes.rs::get_report`) reflects the band's actual movement instead of
/// nothing. Reason is a best-effort read of `world_state` at the moment of
/// the move -- never fabricated, just whichever real signal (disaster/food/
/// water scarcity) was present, falling back to "exploration".
fn track_migration(state: &mut SimulationState, events: &mut Vec<Value>, world_value: &Value, current_day: i32) {
    let alive: Vec<&crate::state::Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    if alive.is_empty() {
        return;
    }
    let n = alive.len() as f64;
    let cx = alive.iter().map(|i| i.x).sum::<f64>() / n;
    let cy = alive.iter().map(|i| i.y).sum::<f64>() / n;

    let (last_x, last_y) = match (state.world_state.last_migration_x, state.world_state.last_migration_y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            state.world_state.last_migration_x = Some(cx);
            state.world_state.last_migration_y = Some(cy);
            state.world_state.last_migration_day = Some(current_day);
            return;
        }
    };
    let last_day = state.world_state.last_migration_day.unwrap_or(current_day);
    if current_day - last_day < MIGRATION_MIN_INTERVAL_DAYS {
        return;
    }
    let dx = cx - last_x;
    let dy = cy - last_y;
    let distance_km = (dx * dx + dy * dy).sqrt() * KM_PER_DEGREE;
    if distance_km < MIGRATION_MIN_KM {
        return;
    }

    let food_abundance = world_value.get("food_abundance").and_then(Value::as_f64).unwrap_or(0.5);
    let water_abundance = world_value.get("water_abundance").and_then(Value::as_f64).unwrap_or(0.5);
    let recent_disaster = world_value.get("recent_disaster").and_then(Value::as_str);
    let reason = if let Some(disaster) = recent_disaster {
        format!("disaster:{disaster}")
    } else if food_abundance < 0.35 {
        "food_scarcity".to_string()
    } else if water_abundance < 0.35 {
        "water_scarcity".to_string()
    } else {
        "exploration".to_string()
    };

    events.push(json!({
        "type": "migration",
        "day": current_day,
        "distance_km": (distance_km * 10.0).round() / 10.0,
        "from": { "x": last_x, "y": last_y },
        "to": { "x": cx, "y": cy },
        "reason": reason,
        "food_abundance": food_abundance,
        "water_abundance": water_abundance,
        "season": world_value.get("season").cloned().unwrap_or(Value::Null),
        "importance": "medium",
    }));

    state.world_state.last_migration_x = Some(cx);
    state.world_state.last_migration_y = Some(cy);
    state.world_state.last_migration_day = Some(current_day);
}

pub fn advance_one_day(state: &mut SimulationState) -> (TickReport, PhaseTimings) {
    let mut phases = PhaseTimings::default();
    let __t_setup = Instant::now();
    state.current_day += 1;
    state.current_year = state.current_day / 365;
    let current_day = state.current_day;

    let mut discovered_techs = as_string_set(&state.discovered_techs);
    let mut discovered_beliefs = as_string_set(&state.discovered_beliefs);
    let mut discovered_arts = as_string_set(&state.discovered_arts);
    let mut astronomy_knowledge = as_string_set(&state.astronomy_knowledge);
    // Snapshot at the *start* of today's tick (yesterday's accumulated
    // knowledge) -- astronomy::process_astronomy_tick below may add new
    // knowledge today, which should only start paying off from tomorrow.
    let astronomy_bonus = astronomy::get_astronomy_bonus(&astronomy_knowledge);
    let farming_bonus = astronomy_bonus.get("farming_efficiency").and_then(Value::as_f64).unwrap_or(0.0);
    let navigation_bonus = astronomy_bonus.get("navigation").and_then(Value::as_f64).unwrap_or(0.0);
    let innovation_bonus = astronomy_bonus.get("innovation_rate").and_then(Value::as_f64).unwrap_or(0.0);
    let mut celestial_observations = as_string_set(&state.celestial_observations);

    // 1. World state (season/weather/resources).
    let mut world_value = serde_json::to_value(&state.world_state).unwrap_or_else(|_| json!({}));
    let alive_count_pre = state.individuals.iter().filter(|i| i.alive && !i.is_dead).count();
    environment::update_world_state(&mut world_value, current_day, Some(&discovered_techs), alive_count_pre);

    // 1b. Natural disaster: low-probability organic event per biome/weather,
    // distinct from god-mode interventions. A pending disaster persists in world
    // state across ticks until processed, mirroring the JS engine's behavior.
    let mut events: Vec<Value> = Vec::new();
    if alive_count_pre >= 4 && world_value.get("natural_disaster").map(Value::is_null).unwrap_or(true)
        && rand::random::<f64>() < environment::natural_disaster_probability(&world_value)
    {
        let (disaster_type, mortality_factor) = environment::pick_natural_disaster(&world_value);
        if let Some(obj) = world_value.as_object_mut() {
            obj.insert("natural_disaster".to_string(), json!({ "type": disaster_type, "mortality_factor": mortality_factor }));
        }
    }
    if let Some(disaster) = world_value.get("natural_disaster").cloned().filter(|v| !v.is_null()) {
        let disaster_type = disaster.get("type").and_then(Value::as_str).unwrap_or("").to_string();
        let mortality_factor = disaster.get("mortality_factor").and_then(Value::as_f64).unwrap_or(0.0);
        events.extend(environment::process_disaster(&disaster_type, mortality_factor, &mut state.individuals, &mut state.groups, current_day));
        if let Some(obj) = world_value.as_object_mut() {
            obj.insert("natural_disaster".to_string(), Value::Null);
            obj.insert("recent_disaster".to_string(), json!(disaster_type));
            obj.insert("recent_disaster_day".to_string(), json!(current_day));
        }
    }
    let recent_disaster_day = world_value.get("recent_disaster_day").and_then(Value::as_i64).unwrap_or(0) as i32;
    if !world_value.get("recent_disaster").map(Value::is_null).unwrap_or(true) && current_day - recent_disaster_day > 120 {
        if let Some(obj) = world_value.as_object_mut() {
            obj.insert("recent_disaster".to_string(), Value::Null);
        }
    }

    state.world_state = serde_json::from_value(world_value.clone()).unwrap_or_default();

    // 1c2. Resource pressure: this settlement's population against this
    // biome's carrying capacity (food_abundance-derived), computed once here
    // rather than per individual since neither the population size nor the
    // world state depends on which individual is gathering. Feeds into
    // gather_resources below, which is what actually caps population growth
    // through the existing satiation -> health -> mortality chain instead of
    // any scripted intervention -- see economy::gather_resources's own
    // comment on food_pressure/water_pressure.
    let resource_pressure = environment::compute_resource_pressure(&world_value, alive_count_pre);
    let food_pressure = resource_pressure.get("food_pressure").and_then(Value::as_f64).unwrap_or(0.0);
    let water_pressure = resource_pressure.get("water_pressure").and_then(Value::as_f64).unwrap_or(0.0);

    // 1c. Phoneme palette: this population's own sound repertoire, derived
    // once from the founders' FOXP2/CNTNAP2 alleles (see
    // language::derive_phoneme_palette). Self-heals states saved before this
    // field existed, matching the lazy-backfill pattern used for inventory
    // below -- everything downstream (vocabulary + name origination) reads
    // it, never a hardcoded pool.
    if state.world_state.phoneme_palette.is_none() {
        state.world_state.phoneme_palette = Some(language::derive_phoneme_palette_from_population(&state.individuals));
    }
    let phoneme_palette = state.world_state.phoneme_palette.clone().unwrap_or_default();

    // 2. Age + generation count (max generation currently alive).
    let updated_age_count: usize = maybe_par_iter_mut!(state
        .individuals)
        .map(|individual| {
            if !individual.alive || individual.is_dead {
                return 0;
            }
            individual.age_days = Some(current_day - individual.birth_day);
            1
        })
        .sum();
    let generation_count = max_alive_generation(&state.individuals);

    // 3. Groups: form/merge bands from proximity before per-individual processing.
    form_groups(state, current_day);

    // 3b. Death witnessing: yesterday's deaths raise fear in nearby/kin
    // survivors and feed real death_of_kin events into today's psychology
    // pass (previously always empty -- see apply_death_witnessing).
    let kin_death_events = apply_death_witnessing(state, current_day);
    phases.setup_ms += __t_setup.elapsed().as_secs_f64() * 1000.0;

    // 4. Per-individual embarrassingly-parallel updates: economy, epigenetics,
    //    consciousness (cardinal rule: only this call may touch mind.consciousness),
    //    psychology, language growth, gut microbiome, decision-making.
    //
    //    Split into several sequential par_iter_mut passes (rather than one
    //    call each in a single pass) purely so the Performance panel can
    //    attribute this block's time to the right engine -- each individual's
    //    per-pass order below is identical to the old single-pass order, so
    //    this is a timing-granularity change only, not a behavior change.
    let group_sizes: HashMap<String, usize> = state
        .groups
        .iter()
        .filter_map(|g| {
            let id = g.get("id")?.as_str()?.to_string();
            let size = g.get("member_ids")?.as_array()?.len();
            Some((id, size))
        })
        .collect();
    let world_value_ref = &world_value;
    let discovered_techs_ref = &discovered_techs;
    let phoneme_palette_ref = &phoneme_palette;

    let __t_economy = Instant::now();
    if !state.disabled_engines.contains("economy") {
        maybe_par_iter_mut!(state.individuals).for_each(|individual| {
            if !individual.alive || individual.is_dead {
                return;
            }
            if individual.inventory.is_empty() {
                individual.inventory = economy::initialize_inventory();
            }
            let gathered = economy::gather_resources(individual, world_value_ref, discovered_techs_ref, farming_bonus, food_pressure, water_pressure);
            for (k, v) in gathered {
                *individual.inventory.entry(k).or_insert(0.0) += v;
            }
            let consumed = economy::consume_resources(individual);
            let satiation = consumed.get("satiation").and_then(Value::as_f64).unwrap_or(0.5);
            individual.extra.insert("satiation".to_string(), json!(satiation));
            let produced = economy::produce_goods(individual, discovered_techs_ref);
            for (k, v) in produced {
                *individual.inventory.entry(k).or_insert(0.0) += v;
            }
            if satiation < 0.3 {
                individual.health.hp = (individual.health.hp - 0.01 * (0.3 - satiation)).max(0.0);
            }
            // weather_hp_delta previously existed as a computed field nothing
            // anywhere ever read -- current_weather never organically changed
            // in the first place, so this was doubly dead. Applied here
            // alongside the other direct per-tick HP adjustments above.
            let weather_hp_delta = world_value_ref.get("weather_hp_delta").and_then(Value::as_f64).unwrap_or(0.0);
            if weather_hp_delta != 0.0 {
                let max_hp = individual.health.max_hp;
                individual.health.hp = (individual.health.hp + weather_hp_delta).clamp(0.0, max_hp);
            }
        });
    }
    phases.economy_ms += __t_economy.elapsed().as_secs_f64() * 1000.0;

    let __t_consciousness_psychology = Instant::now();
    if !state.disabled_engines.contains("consciousness_psychology") {
        maybe_par_iter_mut!(state.individuals).for_each(|individual| {
            if !individual.alive || individual.is_dead {
                return;
            }
            epigenetics::update_epigenome(individual, Some(world_value_ref), current_day);
            // Cardinal rule: consciousness may only be mutated here.
            consciousness::update_consciousness(individual);
            psychology::update_mental_state(individual, &kin_death_events, world_value_ref, current_day);
            // Reads this tick's just-updated stress_level, so must run after
            // update_mental_state above; reads this tick's satiation, which
            // the earlier economy phase already wrote to extra["satiation"].
            hormones::update_hormones(individual, current_day);
            environment::update_water_state(individual);
        });
    }
    phases.consciousness_psychology_ms += __t_consciousness_psychology.elapsed().as_secs_f64() * 1000.0;

    let __t_language_naming = Instant::now();
    if !state.disabled_engines.contains("language_naming") {
        maybe_par_iter_mut!(state.individuals).for_each(|individual| {
            if !individual.alive || individual.is_dead {
                return;
            }
            let group_size = individual
                .group_id
                .as_ref()
                .and_then(|gid| group_sizes.get(gid).copied())
                .unwrap_or(1);
            language::update_foxp2_expression(individual, group_size);
            language::update_language_stage(individual, group_size, generation_count);
            // Vocabulary otherwise could only ever spread by copying an existing
            // teacher's word (learn_from_teacher, below) -- with everyone starting
            // from an empty vocabulary and nothing ever originating a first word,
            // that pathway alone can never bootstrap: it would stay empty for the
            // entire simulation. Each individual gets one chance per tick to
            // originate a word for a random core concept from lived experience;
            // try_acquire_word_from_environment already self-gates on language
            // stage/FOXP2/IQ, so this is a no-op for anyone not ready.
            let concept = language::CORE_CONCEPTS[rand::random::<usize>() % language::CORE_CONCEPTS.len()];
            let group_id = individual.group_id.clone().unwrap_or_default();
            language::try_acquire_word_from_environment(individual, concept, &group_id, phoneme_palette_ref);
            // A personal name is just another word an individual can only
            // originate once their own lived language supports one -- never a
            // birth gift (see naming.rs).
            crate::naming::try_originate_name(individual, &group_id, phoneme_palette_ref);
        });
    }
    phases.language_naming_ms += __t_language_naming.elapsed().as_secs_f64() * 1000.0;

    let __t_microbiome_agent = Instant::now();
    if !state.disabled_engines.contains("microbiome_agent") {
        maybe_par_iter_mut!(state.individuals).for_each(|individual| {
            if !individual.alive || individual.is_dead {
                return;
            }
            // Cardinal rule: mind.inner_thought/inner_thought_log may only be mutated here.
            consciousness::update_inner_thought(individual, current_day);
            microbiome::update_gut_microbiome(individual, world_value_ref);
            // Order matters: select_action must set _currentAction *before*
            // accumulate_experience runs, so experience is gained for today's action
            // rather than lagging one tick behind (matches the JS worker's own ordering).
            let action = agent::select_action(individual, world_value_ref);
            individual.extra.insert("_currentAction".to_string(), json!(action));
            agent::accumulate_experience(individual, world_value_ref);
            biology::reproduction::update_mating_urge(individual, world_value_ref);
        });
    }
    phases.microbiome_agent_ms += __t_microbiome_agent.elapsed().as_secs_f64() * 1000.0;
    // 5. Movement: band cohesion / mating drive / persisted wander heading, gated
    //    by the land mask so nobody drifts into open ocean. Built from a snapshot
    //    taken before anyone moves (standard simultaneous-update flocking).
    //    Filtered to the living: every consumer of this snapshot (apply_movement's
    //    own centroid loop, nearest_fertile_opposite_sex) already excludes dead
    //    individuals via alive/is_dead checks, so cloning+gridding them here too
    //    is pure waste that grows forever with total-ever-born rather than with
    //    current population -- on a decades-old simulation this dwarfed the
    //    living headcount and was the dominant cost in the per-tick Hesaplama
    //    (compute) time.
    let __t_movement = Instant::now();
    if !state.disabled_engines.contains("movement") {
        let pre_move_snapshot: Vec<crate::state::Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).map(movement_stub).collect();
        let pre_move_positions: Vec<(f64, f64)> = pre_move_snapshot.iter().map(|i| (i.x, i.y)).collect();
        let pre_move_grid = SpatialGrid::build(&pre_move_positions, NEARBY_RADIUS);
        let weather_move_mult = world_value.get("weather_move_mult").and_then(Value::as_f64).unwrap_or(1.0);
        apply_movement(state, &pre_move_snapshot, &pre_move_grid, current_day, navigation_bonus, weather_move_mult);
    }
    track_migration(state, &mut events, &world_value, current_day);
    phases.movement_ms += __t_movement.elapsed().as_secs_f64() * 1000.0;

    // 6. Technology + language: observation-based learning only (cardinal rule).
    //    A spatial grid bounds the neighbor search to nearby cells instead of
    //    scanning the whole population for every individual. Filtered to the
    //    living for the same reason as the movement snapshot above -- the loop
    //    below already skips dead individuals as both learner and teacher.
    let __t_observation_learning = Instant::now();
    if !state.disabled_engines.contains("observation_learning") {
        let snapshot: Vec<crate::state::Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).map(observation_stub).collect();
        let positions: Vec<(f64, f64)> = snapshot.iter().map(|i| (i.x, i.y)).collect();
        let grid = SpatialGrid::build(&positions, NEARBY_RADIUS);
        for individual in state.individuals.iter_mut() {
            if !individual.alive || individual.is_dead {
                continue;
            }
            // References into `snapshot`, not clones -- learn_tech_from_observation
            // and learn_from_teacher only ever read `nearby`, so cloning up to
            // MAX_NEARBY_SAMPLE stubs (whose known_techs/vocabulary keep growing
            // across the run) for every single alive individual every tick was
            // pure waste.
            let nearby: Vec<&crate::state::Individual> = grid
                .candidates_within(individual.x, individual.y, NEARBY_RADIUS, MAX_CANDIDATE_SCAN)
                .into_iter()
                .filter_map(|idx| snapshot.get(idx))
                .filter(|other| other.id != individual.id)
                .filter(|other| distance(individual.x, individual.y, other.x, other.y) < NEARBY_RADIUS)
                .take(MAX_NEARBY_SAMPLE)
                .collect();
            if nearby.is_empty() {
                continue;
            }
            technology::learn_tech_from_observation(individual, &nearby, &mut discovered_techs);
            if let Some(&teacher) = nearby.iter().max_by_key(|other| {
                other.language.vocabulary.len()
            }) {
                language::learn_from_teacher(individual, teacher);
            }
            if individual.language.writing {
                if let Some(&scribe) = nearby.iter().find(|other| other.language.writing) {
                    language::read_written_records(individual, scribe);
                }
            }
        }
    }
    phases.observation_learning_ms += __t_observation_learning.elapsed().as_secs_f64() * 1000.0;

    // 6b. Technology emergence: deterministic, purely from an individual's own
    // accumulated physical experience (separate pathway from observation-based
    // learning above; both independently gate on the individual's own known_techs).
    let __t_tech_emergence = Instant::now();
    if !state.disabled_engines.contains("tech_emergence") {
        for individual in state.individuals.iter_mut() {
            if !individual.alive || individual.is_dead {
                continue;
            }
            let emerged = agent::check_tech_emergence(individual, &mut discovered_techs, innovation_bonus);
            for tech_id in emerged {
                events.push(json!({
                    "type": "discovery",
                    "tech_id": tech_id,
                    "discoverer_id": individual.id,
                    "discovery_day": current_day,
                }));
            }
        }
    }
    phases.tech_emergence_ms += __t_tech_emergence.elapsed().as_secs_f64() * 1000.0;
    let __t_reproduction = Instant::now();

    // 7. Reproduction (biology cardinal path: only genetic inheritance + mutation).
    // References, not clones -- check_reproduction only reads this to filter/pair
    // candidates (create_child, the one place needing full genetic data, runs on
    // whichever single pair actually conceives). Cloning every alive individual's
    // full genome/epigenome/inventory/skills/beliefs/language.vocabulary/memory
    // here on every single tick was, by a wide margin, the single most expensive
    // part of the whole tick once population grew into the hundreds.
    //
    // Everything in this block is gated together (the toggle check wraps all
    // of it, down to the birth-processing loop) since it's all one pipeline --
    // but the id->index map immediately after this block stays unconditional,
    // since law/trade downstream need a valid index into state.individuals
    // regardless of whether reproduction ran this tick.
    if !state.disabled_engines.contains("reproduction") {
    // compute_inbreeding_coefficient/coefficient_of_relationship need the
    // ancestor chain up to 10 generations back, which can reach well past
    // whatever window state.individuals is bounded to in memory (see
    // runtime.rs's bounded tick-loop load) -- state.genealogy is the
    // always-fully-loaded index that exists specifically so this stays
    // correct. Backfilling it here from state.individuals (rather than only
    // ever populating it at birth) is what keeps this correct for callers
    // that never pre-load it from the DB at all (the CLI binary, unit/
    // integration tests): it grows in place the same way state.individuals
    // always used to, just without the heavy payload. On the server's tick
    // loop this is normally a no-op -- everyone's already there -- so it
    // only ever does real work the first time an id is seen.
    //
    // This now runs unconditionally, BEFORE check_reproduction, rather than
    // only after a conception happened: conception_probability itself needs
    // each candidate parent's own ancestor chain already indexed so it can
    // compute the *prospective pair's* coefficient of relationship (what F
    // their child would have) -- previously this only ever ran after the
    // fact, once a child already existed, which is too late for the pair's
    // own conception odds to reflect their relatedness.
    for ind in &state.individuals {
        state.genealogy.entry(ind.id.clone()).or_insert_with(|| biology::genome::GenealogyEntry {
            parent_1_id: ind.parent_1_id.clone(),
            parent_2_id: ind.parent_2_id.clone(),
            inbreeding_coeff: ind.inbreeding_coeff.unwrap_or(0.0),
        });
    }
    let alive_snapshot: Vec<&crate::state::Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    let community_lang_stage = state
        .individuals
        .iter()
        .filter(|i| i.alive)
        .map(|i| i.language.stage)
        .max()
        .unwrap_or(0);
    let sim_id = state.id.clone().unwrap_or_default();
    let season = world_value.get("season").and_then(Value::as_str).unwrap_or("spring").to_string();
    let calendar_known = discovered_techs.contains("calendar");
    let mut conceived = biology::reproduction::check_reproduction(
        &alive_snapshot,
        current_day,
        &sim_id,
        community_lang_stage,
        &state.genealogy,
        &season,
        calendar_known,
        &state.groups,
    );
    // Built once and reused by every id-based lookup below (conception and
    // due-birth processing alike): only new children get pushed onto
    // state.individuals in this block, and only at the end, so indices
    // computed here stay valid throughout even as the vector grows.
    let index_by_id: HashMap<String, usize> = state.individuals.iter().enumerate().map(|(i, ind)| (ind.id.clone(), i)).collect();
    if !conceived.is_empty() {
        for child in conceived.iter_mut() {
            // Two owned clones per newborn (not per living individual) --
            // inherit_epigenome needs `&mut Individual` for both parents,
            // so a snapshot is unavoidable here, but index_by_id (already
            // built above, and already how every other id-based lookup in
            // this function works -- see e.g. the due-birth loop just below)
            // gets us there in O(1) instead of paying for a full-population
            // clone map up front.
            let parent1 = child.parent_1_id.as_ref().and_then(|id| index_by_id.get(id)).map(|&idx| state.individuals[idx].clone());
            let parent2 = child.parent_2_id.as_ref().and_then(|id| index_by_id.get(id)).map(|&idx| state.individuals[idx].clone());
            if let (Some(mut p1), Some(mut p2)) = (parent1, parent2) {
                epigenetics::inherit_epigenome(child, &mut p1, &mut p2);
            } else {
                epigenetics::initialize_epigenome(child);
            }
            psychology::initialize_psychology(child);
            child.inbreeding_coeff = Some(biology::genome::compute_inbreeding_coefficient(child, &state.genealogy));
        }
        // Immediate post-conception urge reset (mirrors the old Node engine):
        // the mother's own build-up would otherwise only be reined in
        // indirectly, one tick later, by update_mating_urge's separate
        // pregnancy cap -- and nothing at all currently curbs the father's,
        // who -- unlike the mother -- carries no pregnancy flag update_mating_urge
        // could key off of, so without this he'd keep seeking further
        // conceptions immediately after siring a child.
        for child in &conceived {
            if let Some(idx) = child.parent_1_id.as_ref().and_then(|id| index_by_id.get(id)) {
                let mother = &mut state.individuals[*idx];
                mother.health.pregnancy = Some(current_day);
                mother.extra.insert("mating_urge".to_string(), json!(0.0));
            }
            if let Some(idx) = child.parent_2_id.as_ref().and_then(|id| index_by_id.get(id)) {
                let father = &mut state.individuals[*idx];
                let urge = father.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or(0.0);
                father.extra.insert("mating_urge".to_string(), json!((urge - 0.7).max(0.0)));
            }
        }
        state.pending_births.append(&mut conceived);
    }

    // Only a pregnancy that has reached term becomes a living member of the population.
    let (due, still_pending): (Vec<_>, Vec<_>) = state
        .pending_births
        .drain(..)
        .partition(|child| child.birth_day <= current_day);
    state.pending_births = still_pending;
    for mut child in due {
        let mother_snapshot = child.parent_1_id.as_ref().and_then(|id| index_by_id.get(id)).map(|&idx| state.individuals[idx].clone());
        let father_snapshot = child.parent_2_id.as_ref().and_then(|id| index_by_id.get(id)).map(|&idx| state.individuals[idx].clone());

        // A pregnancy that reaches term after its mother has already died
        // cannot become a live birth -- nothing else in the tick loop ever
        // aborts a pending pregnancy, so without this check a mother who
        // died months into gestation would still "give birth" on schedule
        // to a child whose mother has been dead the entire time.
        let mother_alive = mother_snapshot.as_ref().map(|m| m.alive && !m.is_dead).unwrap_or(false);
        if !mother_alive {
            events.push(json!({ "type": "pregnancy_loss", "individual_id": child.id, "day": current_day, "importance": "low" }));
            continue;
        }

        let mut siblings = Vec::new();

        if let Some(mother_snapshot) = &mother_snapshot {
            let resilience = mother_snapshot.phenotype.health_resilience;
            let max_lifespan = mother_snapshot.phenotype.max_lifespan;
            let mother_risk: f64 = (0.06 * (1.0 - resilience) * (90.0 - max_lifespan.min(90.0)) / 90.0).max(0.002);
            let neonatal_risk = (mother_risk * 0.6).max(0.005);
            if rand::random::<f64>() < neonatal_risk {
                child.alive = false;
                child.is_dead = true;
                child.death_day = Some(current_day);
                child.extra.insert("death_cause".to_string(), json!("birth_complications"));
                events.push(json!({ "type": "death", "individual_id": child.id, "name": individual_display_name(&child), "cause": "birth_complications", "day": current_day, "importance": "medium" }));
            }

            // Twins/triplets: each additional sibling is its own independent
            // conception (fresh gametes), not a clone of the first child.
            if let Some(father_snapshot) = &father_snapshot {
                let fshr = mother_snapshot.phenotype.fertility;
                let twin_chance = (0.003 + (fshr - 0.3) * 0.07).max(0.0);
                if rand::random::<f64>() < twin_chance {
                    let mut twin = biology::individual::create_child(mother_snapshot, father_snapshot, current_day, &sim_id);
                    twin.extra.insert("is_twin".to_string(), json!(true));
                    // The primary child conceived this way gets a real F from
                    // compute_inbreeding_coefficient at conception time; a
                    // twin/triplet created here directly (not via
                    // check_reproduction) must get the same treatment,
                    // otherwise they'd keep create_child's hardcoded 0.0
                    // default forever -- genetically nonsensical for a sibling
                    // born to the same parents at the same instant.
                    twin.inbreeding_coeff = Some(biology::genome::compute_inbreeding_coefficient(&twin, &state.genealogy));
                    // Same reasoning applies to epigenome/psychology: the
                    // primary child gets both at conception time (see the
                    // `conceived` loop above); a twin/triplet created here
                    // directly must get the same treatment or it would keep
                    // an empty epigenome (self-healing to a flat neutral 0.5
                    // at every locus on first tick, instead of a parent-
                    // heritability-weighted blend) and a never-initialized
                    // psychology (permanently blank attachment_style,
                    // hardcoded self_awareness=false).
                    {
                        let mut p1 = mother_snapshot.clone();
                        let mut p2 = father_snapshot.clone();
                        epigenetics::inherit_epigenome(&mut twin, &mut p1, &mut p2);
                    }
                    psychology::initialize_psychology(&mut twin);
                    if rand::random::<f64>() < neonatal_risk * 2.5 {
                        twin.alive = false;
                        twin.is_dead = true;
                        twin.death_day = Some(current_day);
                        twin.extra.insert("death_cause".to_string(), json!("birth_complications"));
                        events.push(json!({ "type": "death", "individual_id": twin.id, "name": individual_display_name(&twin), "cause": "birth_complications", "day": current_day, "importance": "medium" }));
                    }
                    siblings.push(twin);
                    if rand::random::<f64>() < twin_chance * 0.1 {
                        let mut triplet = biology::individual::create_child(mother_snapshot, father_snapshot, current_day, &sim_id);
                        triplet.extra.insert("is_twin".to_string(), json!(true));
                        triplet.inbreeding_coeff = Some(biology::genome::compute_inbreeding_coefficient(&triplet, &state.genealogy));
                        {
                            let mut p1 = mother_snapshot.clone();
                            let mut p2 = father_snapshot.clone();
                            epigenetics::inherit_epigenome(&mut triplet, &mut p1, &mut p2);
                        }
                        psychology::initialize_psychology(&mut triplet);
                        if rand::random::<f64>() < neonatal_risk * 4.0 {
                            triplet.alive = false;
                            triplet.is_dead = true;
                            triplet.death_day = Some(current_day);
                            triplet.extra.insert("death_cause".to_string(), json!("birth_complications"));
                            events.push(json!({ "type": "death", "individual_id": triplet.id, "name": individual_display_name(&triplet), "cause": "birth_complications", "day": current_day, "importance": "medium" }));
                        }
                        siblings.push(triplet);
                    }
                }
            }

            if let Some(mother) = child.parent_1_id.as_ref().and_then(|id| index_by_id.get(id)).map(|&idx| &mut state.individuals[idx]) {
                mother.health.pregnancy = None;
                hormones::apply_birth_surge(mother);
                if rand::random::<f64>() < mother_risk {
                    mother.alive = false;
                    mother.is_dead = true;
                    mother.death_day = Some(current_day);
                    mother.extra.insert("death_cause".to_string(), json!("birth_complications"));
                    events.push(json!({ "type": "death", "individual_id": mother.id, "name": individual_display_name(mother), "cause": "birth_complications", "day": current_day, "importance": "high", "is_founder": mother.is_founder }));
                }
            }

            // Record the mate bond and both parents' children -- previously
            // nothing ever set social.has_mate/mate_id/children_ids, so the
            // client's "Paired"/"N Child" badges (PopulationPanel) could
            // never appear regardless of actual reproduction, and
            // psychology::process_bonding (the only place relationship
            // strength is tracked) was never called from anywhere.
            let child_and_sibling_ids: Vec<String> = std::iter::once(child.id.clone()).chain(siblings.iter().map(|s| s.id.clone())).collect();
            if let (Some(&mother_idx), Some(&father_idx)) = (
                child.parent_1_id.as_ref().and_then(|id| index_by_id.get(id)),
                child.parent_2_id.as_ref().and_then(|id| index_by_id.get(id)),
            ) {
                if mother_idx != father_idx {
                    let (lo, hi) = if mother_idx < father_idx { (mother_idx, father_idx) } else { (father_idx, mother_idx) };
                    let (first_half, second_half) = state.individuals.split_at_mut(hi);
                    let (mother, father) = if mother_idx < father_idx { (&mut first_half[lo], &mut second_half[0]) } else { (&mut second_half[0], &mut first_half[lo]) };
                    mother.social.has_mate = true;
                    mother.social.mate_id = Some(father.id.clone());
                    father.social.has_mate = true;
                    father.social.mate_id = Some(mother.id.clone());
                    mother.social.children_ids.extend(child_and_sibling_ids.iter().cloned());
                    father.social.children_ids.extend(child_and_sibling_ids.iter().cloned());
                    psychology::process_bonding(mother, father, "mating");
                    hormones::apply_mating_surge(mother, father);
                }
            }
        }

        events.push(json!({ "type": "birth", "individual_id": child.id, "day": current_day, "importance": "low" }));
        // Registered in the always-fully-loaded genealogy index (not just
        // state.individuals) so a bounded tick loop can still correctly
        // resolve this child as someone else's ancestor generations from now,
        // long after they've aged out of the in-memory alive+recent window.
        state.genealogy.insert(
            child.id.clone(),
            biology::genome::GenealogyEntry {
                parent_1_id: child.parent_1_id.clone(),
                parent_2_id: child.parent_2_id.clone(),
                inbreeding_coeff: child.inbreeding_coeff.unwrap_or(0.0),
            },
        );
        state.total_ever_born += 1;
        // A newborn inherits `group_id` directly from a parent (see
        // create_child) without ever passing through form_groups's own
        // "ungrouped individual joins a nearby group" path -- that function
        // only ever looks at individuals with group_id == None, so a
        // newborn (already grouped at birth) was silently invisible to it.
        // Nothing else ever appended a birth into its group's member_ids,
        // so group_sizes (built from member_ids.len() a few hundred lines
        // below, and used to gate language-stage advancement, settlement
        // capacity, and every other group-size check) stayed frozen at
        // whatever the founding cluster's size happened to be -- population
        // growing entirely through reproduction (this simulation's only
        // real growth path) could never actually grow its own group_size.
        if let Some(gid) = &child.group_id {
            if let Some(group) = state.groups.iter_mut().find(|g| g.get("id").and_then(Value::as_str) == Some(gid.as_str())) {
                push_member(group, &child.id);
            }
        }
        state.individuals.push(child);
        for sibling in siblings {
            events.push(json!({ "type": "birth", "individual_id": sibling.id, "day": current_day, "importance": "low" }));
            state.genealogy.insert(
                sibling.id.clone(),
                biology::genome::GenealogyEntry {
                    parent_1_id: sibling.parent_1_id.clone(),
                    parent_2_id: sibling.parent_2_id.clone(),
                    inbreeding_coeff: sibling.inbreeding_coeff.unwrap_or(0.0),
                },
            );
            state.total_ever_born += 1;
            if let Some(gid) = &sibling.group_id {
                if let Some(group) = state.groups.iter_mut().find(|g| g.get("id").and_then(Value::as_str) == Some(gid.as_str())) {
                    push_member(group, &sibling.id);
                }
            }
            state.individuals.push(sibling);
        }
    }
    }

    // Nothing after this point pushes/removes elements from state.individuals
    // this tick (only the birth processing above does, and that's already
    // finished), so one id->index map built here stays valid for every
    // remaining lookup this tick -- reused below by both the law section and
    // the trade section, which previously each rebuilt their own copy.
    let index_by_id: HashMap<String, usize> = state.individuals.iter().enumerate().map(|(i, ind)| (ind.id.clone(), i)).collect();
    phases.reproduction_ms += __t_reproduction.elapsed().as_secs_f64() * 1000.0;
    // 8. Mortality.
    let __t_mortality_roll = Instant::now();
    if !state.disabled_engines.contains("mortality_roll") {
        for individual in state.individuals.iter_mut() {
            if !individual.alive || individual.is_dead {
                continue;
            }
            let roll_death_cause = biology::mortality::roll_death(individual, current_day, Some(&world_value));
            // Only reached by an individual who just survived today's
            // roll_death check above -- see wounds.rs's own doc comment for
            // why a wound is deliberately a consequence of danger that
            // *didn't* kill via that unrelated roll, not an independent
            // risk on top of it. Healing runs for every survivor regardless
            // of whether they were wounded today (so an existing wound from
            // an earlier tick keeps closing even on a day with no new
            // danger).
            let wound_collapse_cause = if roll_death_cause.is_none() {
                biology::wounds::maybe_inflict_wound(individual, current_day, Some(&world_value));
                biology::wounds::update_wound_healing(individual);
                biology::wounds::wound_collapse_cause(individual)
            } else {
                None
            };
            if let Some(cause) = roll_death_cause.or(wound_collapse_cause) {
                individual.alive = false;
                individual.is_dead = true;
                individual.death_day = Some(current_day);
                // snake_case, not the raw Debug PascalCase, so it matches every
                // other death-cause writer in this codebase (conflict/infection/
                // disaster/birth_complications) and the client's CAUSE_LABELS
                // i18n lookup, which is keyed on snake_case and otherwise
                // silently fell back to showing the raw English enum name.
                let cause_str = pascal_to_snake(&format!("{cause:?}"));
                individual.extra.insert("death_cause".to_string(), json!(cause_str));
                events.push(json!({ "type": "death", "individual_id": individual.id, "name": individual_display_name(individual), "cause": cause_str, "day": current_day, "importance": "medium", "is_founder": individual.is_founder }));
            }
        }
    }
    phases.mortality_roll_ms += __t_mortality_roll.elapsed().as_secs_f64() * 1000.0;

    // 9. Microbiome outbreaks (population-wide contagion) -- can itself kill
    // via infection, so today's dead aren't all known until after this runs.
    let __t_microbiome_outbreak = Instant::now();
    if !state.disabled_engines.contains("microbiome_outbreak") {
        events.extend(microbiome::process_microbiome_tick(&mut state.individuals, &world_value, current_day));
    }
    phases.microbiome_outbreak_ms += __t_microbiome_outbreak.elapsed().as_secs_f64() * 1000.0;

    // World state's own `disease_pressure` (read by mortality::compute_daily_death_risk
    // as a small background risk contribution) used to be written once at
    // simulation creation and never again, staying flat at its initial 0.1
    // for the entire run regardless of whether an actual epidemic was
    // underway. Deriving it here from this tick's real infected fraction
    // (written into state.world_state directly, since world_value's own
    // conversion back into state.world_state already happened earlier this
    // tick, before microbiome processing) means a real outbreak now raises
    // background mortality risk, and an outbreak's end lets it fall again,
    // instead of an unchanging constant.
    let alive_after_microbiome = state.individuals.iter().filter(|i| i.alive && !i.is_dead).count();
    if alive_after_microbiome > 0 {
        let infected_count = state
            .individuals
            .iter()
            .filter(|i| i.alive && !i.is_dead)
            .filter(|i| i.extra.get("infections").and_then(Value::as_array).map(|arr| !arr.is_empty()).unwrap_or(false))
            .count();
        let disease_pressure = infected_count as f64 / alive_after_microbiome as f64;
        state.world_state.extra.insert("disease_pressure".to_string(), json!(disease_pressure));
    }

    // 9b. Prune today's dead out of their group's member_ids. Unlike
    // environment::process_disaster (which already does this for disaster
    // deaths) and law's exile enforcement, none of ordinary mortality (8),
    // infection (9), or a birth-complication death during reproduction (7)
    // ever removed the individual from their group -- member_ids only ever
    // grew. That has two costs: group "size" (member_ids.len(), used by
    // settlement-formation thresholds and every per-group pass below) was
    // silently inflated by long-dead members, and a group whose every named
    // member has actually died never reads as empty, so form_groups's own
    // `state.groups.retain(...)` (next tick) could never drop it -- groups
    // (and the law/social/culture/architecture work done per group every
    // tick) accumulated forever even while the living population stayed
    // flat or shrank.
    let __t_group_pruning = Instant::now();
    if !state.disabled_engines.contains("group_pruning") {
        let newly_dead_ids: Vec<String> = state
            .individuals
            .iter()
            .filter(|i| i.is_dead && i.death_day == Some(current_day))
            .map(|i| i.id.clone())
            .collect();
        prune_dead_from_groups(&mut state.groups, &newly_dead_ids);
    }
    phases.group_pruning_ms += __t_group_pruning.elapsed().as_secs_f64() * 1000.0;
    // 10. Belief formation (per-individual reflection) + spread + ritual emergence
    //     + labeling.
    let __t_belief = Instant::now();
    if !state.disabled_engines.contains("belief") {
        for individual in state.individuals.iter_mut() {
            if !individual.alive || individual.is_dead {
                continue;
            }
            if let Some(ev) = belief::try_form_belief(individual, &mut discovered_beliefs, &discovered_techs, &world_value, current_day) {
                events.push(ev);
            }
        }
        events.extend(belief::update_belief_spread(&mut state.individuals, &discovered_beliefs, &mut state.groups, current_day));
        // group_id -> members via two O(population)-scale passes, shared across
        // every group below, instead of each check_ritual_emergence call
        // filtering the *entire* population itself -- an O(groups * population)
        // cost that got sharply worse as both grew over a long run.
        let ritual_members_by_group: HashMap<String, Vec<&crate::state::Individual>> = {
            let mut individual_group: HashMap<&str, &str> = HashMap::new();
            for g in state.groups.iter() {
                let Some(gid) = g.get("id").and_then(Value::as_str) else { continue };
                let Some(ids) = g.get("member_ids").and_then(Value::as_array) else { continue };
                for id in ids.iter().filter_map(Value::as_str) {
                    individual_group.insert(id, gid);
                }
            }
            let mut by_group: HashMap<String, Vec<&crate::state::Individual>> = HashMap::new();
            for ind in state.individuals.iter() {
                if let Some(&gid) = individual_group.get(ind.id.as_str()) {
                    by_group.entry(gid.to_string()).or_default().push(ind);
                }
            }
            by_group
        };
        for group in state.groups.iter_mut() {
            let group_id = group.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let members = ritual_members_by_group.get(&group_id).map(Vec::as_slice).unwrap_or(&[]);
            if let Some(ev) = belief::check_ritual_emergence(group, members, &discovered_beliefs, current_day) {
                events.push(ev);
            }
        }

        // 10b. Belief labeling: a discovered belief only gets a player-facing
        // name once someone who holds it has actually reached proto-words
        // (stage 3) -- see belief::try_label_belief. Two passes to avoid
        // borrowing state.individuals and state.belief_labels mutably at once.
        let newly_labeled: Vec<(String, String)> = discovered_beliefs
            .iter()
            .filter(|id| !state.belief_labels.contains_key(*id))
            .filter_map(|id| belief::try_label_belief(id, &state.individuals, &phoneme_palette).map(|label| (id.clone(), label)))
            .collect();
        for (belief_id, label) in newly_labeled {
            events.push(json!({ "type": "belief_named", "belief_id": belief_id, "label": label, "day": current_day, "importance": "medium" }));
            state.belief_labels.insert(belief_id, label);
        }
    }
    phases.belief_ms += __t_belief.elapsed().as_secs_f64() * 1000.0;

    // 11. Culture, civilization naming, art.
    let __t_culture_art = Instant::now();
    if !state.disabled_engines.contains("culture_art") {
        events.extend(culture::process_culture_tick(&state.individuals, &mut state.groups, &discovered_techs, current_day, &phoneme_palette));

        // 11b. Civilization naming: see culture::try_name_civilization.
        if state.civilization_name.is_none() {
            if let Some(name) = culture::try_name_civilization(&state.individuals, &state.groups, &phoneme_palette) {
                events.push(json!({ "type": "civilization_named", "name": name, "day": current_day, "importance": "high" }));
                state.civilization_name = Some(name);
            }
        }
        events.extend(art::process_art_tick(&state.individuals, &mut discovered_arts, &discovered_techs, &world_value, current_day));
        for individual in state.individuals.iter_mut() {
            if individual.is_dead {
                continue;
            }
            // Previously always called with `group: None`, which meant
            // apply_art_effects's own documented group-tension-reduction
            // effect (>3 discovered art forms cooling a group down) was
            // permanently dead code in the running simulation -- it only
            // ever exercised in unit tests that constructed a synthetic
            // group directly. Look up the individual's actual group (a
            // disjoint field of `state`, not a method borrow, so this
            // coexists fine with the `state.individuals` iteration above).
            let group = individual.group_id.as_deref().and_then(|gid| state.groups.iter_mut().find(|g| g.get("id").and_then(Value::as_str) == Some(gid)));
            art::apply_art_effects(individual, group, &discovered_arts);
        }
    }
    phases.culture_art_ms += __t_culture_art.elapsed().as_secs_f64() * 1000.0;

    // 12. Social: leadership + roles + fission signalling.
    let __t_social = Instant::now();
    if !state.disabled_engines.contains("social") {
        social::apply_resource_tension(&mut state.groups, food_pressure, water_pressure);
        events.extend(social::process_group_dynamics(&mut state.individuals, &mut state.groups, current_day));
        let leader_by_group: HashMap<String, String> = state
            .groups
            .iter()
            .filter_map(|g| {
                let id = g.get("id")?.as_str()?.to_string();
                let leader = g.get("leader_id")?.as_str()?.to_string();
                Some((id, leader))
            })
            .collect();
        // Snapshot of each current group leader's own behavior tally, built
        // before the mutable pass below so a juvenile can observe their
        // leader-parent's pattern without a self-referential borrow of
        // state.individuals (see social::observe_leadership_style).
        let leader_behavior_by_id: HashMap<String, Value> = state
            .individuals
            .iter()
            .filter(|i| i.group_id.as_ref().and_then(|gid| leader_by_group.get(gid)).is_some_and(|lid| lid == &i.id))
            .map(|i| (i.id.clone(), i.extra.get("_behaviorCounts").cloned().unwrap_or_else(|| json!({}))))
            .collect();
        maybe_par_iter_mut!(state.individuals).for_each(|individual| {
            if individual.is_dead {
                return;
            }
            let leader_id = individual.group_id.as_ref().and_then(|gid| leader_by_group.get(gid));
            let role = social::compute_role_for(individual, leader_id.map(|s| s.as_str()));
            individual.extra.insert("group_role".to_string(), json!(role));
            social::observe_leadership_style(individual, &leader_behavior_by_id, JUVENILE_MAX_AGE_YEARS);
        });
    }
    phases.social_ms += __t_social.elapsed().as_secs_f64() * 1000.0;

    // 13. Law: norm emergence, then per-member violation checks (cardinal rule:
    //     driven by the violator's own phenotype, never a random external pick)
    //     and exile enforcement once a group has adopted punishment_exile.
    //     Reuses the id->index map built above instead of a linear .find() over
    //     the entire population for every member of every group -- an O(members
    //     * population) scan per tick that got sharply worse as both grew.
    let __t_law = Instant::now();
    if !state.disabled_engines.contains("law") {
        // Rebuilt fresh here (not reusing step 10's ritual_members_by_group)
        // because step 12's fission may have moved individuals between groups
        // and created new groups since then. Same two-pass O(population)
        // technique as step 10, replacing an O(groups * population) scan.
        let law_members_by_group: HashMap<String, Vec<&crate::state::Individual>> = {
            let mut individual_group: HashMap<&str, &str> = HashMap::new();
            for g in state.groups.iter() {
                let Some(gid) = g.get("id").and_then(Value::as_str) else { continue };
                let Some(ids) = g.get("member_ids").and_then(Value::as_array) else { continue };
                for id in ids.iter().filter_map(Value::as_str) {
                    individual_group.insert(id, gid);
                }
            }
            let mut by_group: HashMap<String, Vec<&crate::state::Individual>> = HashMap::new();
            for ind in state.individuals.iter().filter(|i| !i.is_dead) {
                if let Some(&gid) = individual_group.get(ind.id.as_str()) {
                    by_group.entry(gid.to_string()).or_default().push(ind);
                }
            }
            by_group
        };
        // Norm emergence first, while law_members_by_group's immutable borrow of
        // state.individuals is still alive -- enforcement below needs `&mut
        // state.individuals[idx]`, which can't coexist with that borrow, so it
        // runs as a second pass after law_members_by_group is out of scope.
        for group in state.groups.iter_mut() {
            let group_id = group.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let members = law_members_by_group.get(&group_id).map(Vec::as_slice).unwrap_or(&[]);
            events.extend(law::process_law_tick(group, members, &discovered_techs, current_day));
        }
        drop(law_members_by_group);

        for group in state.groups.iter_mut() {
            let norms: HashSet<String> = group
                .get("norms")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default();
            if norms.is_empty() {
                continue;
            }
            let member_ids: Vec<String> = group
                .get("member_ids")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default();
            for member_id in member_ids {
                let Some(&idx) = index_by_id.get(member_id.as_str()) else { continue };
                let individual = &mut state.individuals[idx];
                if individual.is_dead {
                    continue;
                }
                if let Some(violated_norm) = law::check_norm_violation(individual, &norms) {
                    events.push(law::process_norm_enforcement(group, individual, violated_norm, current_day));
                }
            }
        }
    }
    phases.law_ms += __t_law.elapsed().as_secs_f64() * 1000.0;

    // 14. Architecture: form settlements once a group is large enough, then
    //     build. 14b. Intergroup conflict: rare raids between already-hostile
    //     (post-fission rival) groups.
    let __t_architecture_conflict = Instant::now();
    if !state.disabled_engines.contains("architecture_conflict") {
        prune_orphaned_settlements(&mut state.settlements, &state.groups);
        let mut new_settlements = Vec::new();
        for group in state.groups.iter() {
            let Some(group_id) = group.get("id").and_then(Value::as_str) else { continue };
            let member_count = group.get("member_ids").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
            let has_settlement = state.settlements.iter().any(|s| s.get("group_id").and_then(Value::as_str) == Some(group_id));
            if member_count >= 4 && !has_settlement {
                new_settlements.push(architecture::create_settlement(group, &world_value, current_day));
            }
        }
        state.settlements.append(&mut new_settlements);
        // group_id -> members (alive only, by individual.group_id -- matching
        // process_architecture_tick's own prior filter) via a single mutable
        // pass, instead of filtering the *entire* population once per
        // settlement -- an O(settlements * population) cost that got sharply
        // worse as both grew over a long run. Each individual lands in at most
        // one bucket, so collecting multiple &mut Individual this way is safe.
        let mut architecture_members_by_group: HashMap<String, Vec<&mut crate::state::Individual>> = HashMap::new();
        for ind in state.individuals.iter_mut().filter(|i| !i.is_dead) {
            if let Some(gid) = ind.group_id.clone() {
                architecture_members_by_group.entry(gid).or_default().push(ind);
            }
        }
        for settlement in state.settlements.iter_mut() {
            let group_id = settlement.get("group_id").and_then(Value::as_str).unwrap_or("").to_string();
            let mut members = architecture_members_by_group.remove(&group_id).unwrap_or_default();
            events.extend(architecture::process_architecture_tick(settlement, &mut members, &discovered_techs, &world_value, current_day));
        }
        let group_sizes_for_settlements: HashMap<String, usize> = state
            .groups
            .iter()
            .filter_map(|g| Some((g.get("id")?.as_str()?.to_string(), g.get("member_ids")?.as_array()?.len())))
            .collect();
        for settlement in state.settlements.iter_mut() {
            let group_size = settlement
                .get("group_id")
                .and_then(Value::as_str)
                .and_then(|gid| group_sizes_for_settlements.get(gid).copied())
                .unwrap_or(0);
            if let Some(ev) = architecture::check_settlement_overcrowding(settlement, group_size, current_day) {
                if let Some(gid) = settlement.get("group_id").and_then(Value::as_str) {
                    social::apply_overcrowding_tension(&mut state.groups, gid);
                }
                events.push(ev);
            }
        }

        events.extend(social::process_intergroup_conflict(&mut state.individuals, &mut state.groups, &state.settlements, current_day));
    }
    phases.architecture_conflict_ms += __t_architecture_conflict.elapsed().as_secs_f64() * 1000.0;

    // 15. Astronomy.
    let __t_astronomy = Instant::now();
    if !state.disabled_engines.contains("astronomy") {
        events.extend(astronomy::process_astronomy_tick(
            &state.individuals,
            &mut celestial_observations,
            &mut astronomy_knowledge,
            &discovered_techs,
            current_day,
        ));
    }
    phases.astronomy_ms += __t_astronomy.elapsed().as_secs_f64() * 1000.0;

    // 16. Trade between nearby living individuals (cheap pairing pass) + disease spread.
    let __t_trade_disease = Instant::now();
    if !state.disabled_engines.contains("trade_disease") {
        // Pairing must reflect physical proximity -- a trade partner met in
        // person, a pathogen passed to a *nearby* susceptible other (see
        // microbiome::spread_infection's own doc comment) -- not storage-order
        // adjacency. `alive_ids.chunks(2)` previously paired individuals by
        // their position in state.individuals (roughly birth order), so two
        // people on opposite sides of the map after a group fission/migration
        // could trade or transmit disease every tick with zero distance
        // gating. A spatial grid bounds each individual's partner search to
        // nearby cells (same MAX_CANDIDATE_SCAN cap used by mating/
        // observation-learning above) instead of a full O(n^2) scan, and each
        // individual is paired with at most one nearby partner per tick,
        // preserving the original "cheap single pairing pass" shape.
        let alive: Vec<(String, f64, f64)> = state.individuals.iter().filter(|i| i.alive).map(|i| (i.id.clone(), i.x, i.y)).collect();
        let positions: Vec<(f64, f64)> = alive.iter().map(|&(_, x, y)| (x, y)).collect();
        let grid = SpatialGrid::build(&positions, NEARBY_RADIUS);
        let mut paired = vec![false; alive.len()];
        for i in 0..alive.len() {
            if paired[i] {
                continue;
            }
            let (ax, ay) = positions[i];
            let Some(j) = grid
                .candidates_within(ax, ay, NEARBY_RADIUS, MAX_CANDIDATE_SCAN)
                .into_iter()
                .find(|&j| j != i && !paired[j] && distance(ax, ay, positions[j].0, positions[j].1) < NEARBY_RADIUS)
            else {
                continue;
            };
            paired[i] = true;
            paired[j] = true;
            let (Some(ia), Some(ib)) = (index_by_id.get(&alive[i].0).copied(), index_by_id.get(&alive[j].0).copied()) else {
                continue;
            };
            if ia == ib {
                continue;
            }
            let (lo, hi) = if ia < ib { (ia, ib) } else { (ib, ia) };
            let (left, right) = state.individuals.split_at_mut(hi);
            if let Some(ev) = economy::attempt_trade(&mut left[lo], &mut right[0], current_day) {
                events.push(ev);
            }
            // Disease spread — unconditional in both directions; spread_infection
            // itself enforces immunity/duplicate-infection checks.
            let pathogens_a: Vec<String> = left[lo]
                .extra
                .get("infections")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|inf| inf.get("pathogen_id").and_then(Value::as_str).map(String::from)).collect())
                .unwrap_or_default();
            for pathogen_id in &pathogens_a {
                microbiome::spread_infection(&left[lo], &mut right[0], pathogen_id, current_day, alive.len());
            }
            let pathogens_b: Vec<String> = right[0]
                .extra
                .get("infections")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|inf| inf.get("pathogen_id").and_then(Value::as_str).map(String::from)).collect())
                .unwrap_or_default();
            for pathogen_id in &pathogens_b {
                microbiome::spread_infection(&right[0], &mut left[lo], pathogen_id, current_day, alive.len());
            }
        }
    }
    phases.trade_disease_ms += __t_trade_disease.elapsed().as_secs_f64() * 1000.0;

    // Persist discovered sets back onto the state.
    state.discovered_techs = from_string_set(discovered_techs);
    state.discovered_beliefs = from_string_set(discovered_beliefs);
    state.discovered_arts = from_string_set(discovered_arts);
    state.astronomy_knowledge = from_string_set(astronomy_knowledge);
    state.celestial_observations = from_string_set(celestial_observations);

    let alive_count = state.alive_count();
    state.world_state.alive_count = Some(alive_count);
    state.world_state.current_day = Some(state.current_day);
    state.world_state.current_year = Some(state.current_year);

    for individual in state.individuals.iter_mut() {
        strip_dead_individual_if_due(individual, current_day);
    }

    // Civilization-milestone events (population/tech/belief/art/language-stage/
    // longevity firsts) -- each key fires at most once, tracked in state.milestones.
    let max_alive_language_stage = state.individuals.iter().filter(|i| i.alive && !i.is_dead).map(|i| i.language.stage).max().unwrap_or(0);
    let mut fired_milestones = as_string_set(&state.milestones);
    events.extend(milestones::check_milestones(
        alive_count,
        state.discovered_techs.len(),
        state.discovered_beliefs.len(),
        state.discovered_arts.len(),
        max_alive_language_stage,
        current_day,
        &mut fired_milestones,
    ));
    state.milestones = from_string_set(fired_milestones);

    // Writing societies "record" today's most notable event for posterity --
    // any literate group member can later access it via
    // language::read_written_records, even without having witnessed it
    // themselves (see language.rs for the observational-learning rationale).
    if let Some(notable) = events.last().cloned() {
        for ind in state.individuals.iter_mut().filter(|i| i.alive && i.language.writing) {
            language::record_event_for_posterity(ind, &notable, current_day);
        }
    }

    // Every death-producing path in this tick (mortality/birth-complications
    // above, plus disaster/infection/conflict merged in via events.extend
    // earlier) pushes a "death" event -- counted here into the same
    // dedicated, monotonic `total_ever_born`-style counter (see its own doc
    // comment on state.rs) rather than ever re-deriving a death total by
    // counting `individuals`, which is bounded (alive+recently-dead only)
    // on several read paths and would silently undercount.
    state.total_ever_died += events.iter().filter(|e| e.get("type").and_then(Value::as_str) == Some("death")).count() as i32;

    if !events.is_empty() {
        state.events.extend(events);
        let len = state.events.len();
        if len > MAX_EVENTS {
            state.events.drain(0..len - MAX_EVENTS);
        }
    }

    (
        TickReport {
            current_day: state.current_day,
            alive_count,
            updated_age_count,
        },
        phases,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Individual;

    fn ind_with_generation(generation: i32, alive: bool, is_dead: bool) -> Individual {
        Individual { generation: Some(generation), alive, is_dead, ..Default::default() }
    }

    fn alive_at(id: &str, x: f64, y: f64) -> Individual {
        Individual { id: id.to_string(), x, y, alive: true, ..Default::default() }
    }

    #[test]
    fn track_migration_only_establishes_a_baseline_on_the_first_call() {
        let mut state = SimulationState { individuals: vec![alive_at("a", 10.0, 20.0)], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        assert!(events.is_empty());
        assert_eq!(state.world_state.last_migration_x, Some(10.0));
        assert_eq!(state.world_state.last_migration_y, Some(20.0));
        assert_eq!(state.world_state.last_migration_day, Some(1));
    }

    #[test]
    fn track_migration_stays_silent_below_the_minimum_distance() {
        let mut state = SimulationState { individuals: vec![alive_at("a", 10.0, 20.0)], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        state.individuals[0].x += 0.001; // well under MIGRATION_MIN_KM
        track_migration(&mut state, &mut events, &json!({}), 1 + MIGRATION_MIN_INTERVAL_DAYS);
        assert!(events.is_empty());
    }

    #[test]
    fn track_migration_stays_silent_before_the_minimum_interval_elapses() {
        let mut state = SimulationState { individuals: vec![alive_at("a", 10.0, 20.0)], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        state.individuals[0].x += 1.0; // well over MIGRATION_MIN_KM in degrees
        track_migration(&mut state, &mut events, &json!({}), 1 + MIGRATION_MIN_INTERVAL_DAYS - 1);
        assert!(events.is_empty(), "a real relocation shouldn't log before the minimum interval");
    }

    #[test]
    fn track_migration_logs_a_real_relocation_and_resets_the_baseline() {
        let mut state = SimulationState { individuals: vec![alive_at("a", 10.0, 20.0)], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        state.individuals[0].x += 1.0;
        let day = 1 + MIGRATION_MIN_INTERVAL_DAYS;
        track_migration(&mut state, &mut events, &json!({ "food_abundance": 0.8, "water_abundance": 0.8, "season": "summer" }), day);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev["type"], "migration");
        assert_eq!(ev["day"], day);
        assert!(ev["distance_km"].as_f64().unwrap() > MIGRATION_MIN_KM);
        assert_eq!(ev["reason"], "exploration");
        assert_eq!(state.world_state.last_migration_x, Some(11.0));
        assert_eq!(state.world_state.last_migration_day, Some(day));
    }

    #[test]
    fn track_migration_attributes_reason_to_food_scarcity_when_present() {
        let mut state = SimulationState { individuals: vec![alive_at("a", 0.0, 0.0)], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        state.individuals[0].x += 1.0;
        track_migration(&mut state, &mut events, &json!({ "food_abundance": 0.1 }), 1 + MIGRATION_MIN_INTERVAL_DAYS);
        assert_eq!(events[0]["reason"], "food_scarcity");
    }

    #[test]
    fn track_migration_ignores_dead_and_empty_populations() {
        let mut state = SimulationState { individuals: vec![], ..Default::default() };
        let mut events = Vec::new();
        track_migration(&mut state, &mut events, &json!({}), 1);
        assert!(state.world_state.last_migration_x.is_none(), "no living individuals means no baseline yet");
        assert!(events.is_empty());
    }

    #[test]
    fn h16_regression_dead_high_generation_individual_is_excluded() {
        let dead = ind_with_generation(4, false, true);
        let alive = ind_with_generation(0, true, false);
        assert_eq!(max_alive_generation(&[dead, alive]), 0);
    }

    #[test]
    fn max_generation_across_all_alive_individuals_is_returned() {
        let gen2 = ind_with_generation(2, true, false);
        let gen5 = ind_with_generation(5, true, false);
        assert_eq!(max_alive_generation(&[gen2, gen5]), 5);
    }

    #[test]
    fn empty_population_yields_generation_zero() {
        assert_eq!(max_alive_generation(&[]), 0);
    }

    #[test]
    fn event_log_is_trimmed_to_max_events() {
        let mut state = crate::state::SimulationState {
            events: (0..999).map(|i| json!({ "type": "test", "n": i })).collect(),
            ..Default::default()
        };
        state.individuals.push(crate::biology::individual::create_founder(&json!({ "sex": "male", "ageYears": 25, "x": 0, "y": 0 })));
        state.individuals.push(crate::biology::individual::create_founder(&json!({ "sex": "female", "ageYears": 25, "x": 0, "y": 0 })));
        advance_one_day(&mut state);
        assert!(state.events.len() <= MAX_EVENTS);
    }

    #[test]
    fn push_member_appends_in_place_without_a_redundant_duplicate_scan() {
        let mut group = json!({ "id": "g1", "member_ids": ["a", "b"] });
        push_member(&mut group, "c");
        assert_eq!(group["member_ids"], json!(["a", "b", "c"]));
    }

    // Regression test for the O(group_size^2) bug push_member used to have:
    // every join cloned the whole member_ids array (and, before that, also
    // linearly rescanned it for a duplicate that structurally can't occur --
    // see push_member's own comment). This exercises the exact call path,
    // form_groups's "join an existing group" loop, with enough joins that
    // the removed debug_assert would catch a wrong invariant immediately in
    // a test build if this fix were ever unsafe to make.
    #[test]
    fn many_individuals_join_the_same_existing_group_without_duplication() {
        let mut state = crate::state::SimulationState {
            groups: vec![json!({
                "id": "g1", "member_ids": [], "leader_id": Value::Null, "founded_day": 0,
                "territory": { "x": 0.0, "y": 0.0 }, "internal_tension": 0.3, "norms": [], "culture": [],
            })],
            ..Default::default()
        };
        for i in 0..200 {
            state.individuals.push(Individual { id: format!("p{i}"), alive: true, is_dead: false, x: 0.1, y: 0.1, ..Default::default() });
        }
        form_groups(&mut state, 1);
        let member_ids = state.groups[0]["member_ids"].as_array().expect("member_ids array");
        assert_eq!(member_ids.len(), 200);
        for i in 0..200 {
            assert!(member_ids.iter().any(|v| v.as_str() == Some(&format!("p{i}"))));
            assert_eq!(state.individuals[i].group_id.as_deref(), Some("g1"));
        }
    }

    // ── strip_dead_individual_if_due() ────────────────────────────────────

    fn dead_individual_with_full_data(death_day: i32) -> Individual {
        let mut ind = Individual { is_dead: true, alive: false, death_day: Some(death_day), ..Default::default() };
        ind.genome.insert("locus1".to_string(), Default::default());
        ind.epigenome.insert("locus1".to_string(), Default::default());
        ind.inventory.insert("food".to_string(), 5.0);
        ind.skills.push(json!({ "id": "farming" }));
        ind.beliefs.insert("belief_1".to_string());
        ind.memory = json!({ "events": ["birth"] });
        ind.known_techs.push("fire".to_string());
        ind
    }

    #[test]
    fn long_dead_individual_has_heavy_fields_stripped() {
        let mut ind = dead_individual_with_full_data(0);
        strip_dead_individual_if_due(&mut ind, DEAD_FIELD_STRIP_GRACE_DAYS);
        assert!(ind.genome.is_empty());
        assert!(ind.epigenome.is_empty());
        assert!(ind.inventory.is_empty());
        assert!(ind.skills.is_empty());
        assert!(ind.beliefs.is_empty());
        assert!(ind.known_techs.is_empty());
        assert_eq!(ind.memory, Value::Null);
    }

    #[test]
    fn recently_dead_individual_within_grace_window_is_untouched() {
        let mut ind = dead_individual_with_full_data(0);
        strip_dead_individual_if_due(&mut ind, DEAD_FIELD_STRIP_GRACE_DAYS - 1);
        assert!(!ind.genome.is_empty());
        assert!(!ind.epigenome.is_empty());
        assert!(!ind.inventory.is_empty());
    }

    #[test]
    fn alive_individual_is_never_stripped() {
        let mut ind = dead_individual_with_full_data(0);
        ind.is_dead = false;
        ind.alive = true;
        strip_dead_individual_if_due(&mut ind, 1000);
        assert!(!ind.genome.is_empty());
    }

    #[test]
    fn stripping_preserves_phenotype_mind_language_and_health() {
        let mut ind = dead_individual_with_full_data(0);
        ind.phenotype.height_factor = 0.8;
        ind.mind.consciousness = 0.9;
        ind.language.stage = 3;
        ind.health.hp = 0.4;
        strip_dead_individual_if_due(&mut ind, DEAD_FIELD_STRIP_GRACE_DAYS);
        assert_eq!(ind.phenotype.height_factor, 0.8);
        assert_eq!(ind.mind.consciousness, 0.9);
        assert_eq!(ind.language.stage, 3);
        assert_eq!(ind.health.hp, 0.4);
    }

    #[test]
    fn already_stripped_individual_is_left_alone_on_repeat_runs() {
        let mut ind = dead_individual_with_full_data(0);
        strip_dead_individual_if_due(&mut ind, DEAD_FIELD_STRIP_GRACE_DAYS);
        ind.social.reputation = 0.5;
        strip_dead_individual_if_due(&mut ind, DEAD_FIELD_STRIP_GRACE_DAYS + 10);
        assert_eq!(ind.social.reputation, 0.5, "already-stripped individuals are a no-op, not re-defaulted every call");
    }

    #[test]
    fn individual_missing_death_day_is_never_stripped() {
        let mut ind = dead_individual_with_full_data(0);
        ind.death_day = None;
        strip_dead_individual_if_due(&mut ind, 10_000);
        assert!(!ind.genome.is_empty());
    }

    // ── movement_stub() ─────────────────────────────────────────────────

    #[test]
    fn movement_stub_carries_only_what_apply_movement_and_mate_search_read() {
        let mut ind = dead_individual_with_full_data(0); // reuse the "fully loaded" builder for its heavy fields
        ind.id = "elder".to_string();
        ind.x = 12.0;
        ind.y = -4.0;
        ind.sex = "female".to_string();
        ind.birth_day = -365;
        ind.group_id = Some("g1".to_string());

        let stub = movement_stub(&ind);
        assert_eq!(stub.id, "elder");
        assert_eq!(stub.x, 12.0);
        assert_eq!(stub.y, -4.0);
        assert_eq!(stub.sex, "female");
        assert_eq!(stub.birth_day, -365);
        assert_eq!(stub.group_id, Some("g1".to_string()));
        assert!(stub.alive);
        assert!(!stub.is_dead);

        // The whole point: none of the heavy per-individual maps/vecs this
        // builder populated should have been cloned along with it.
        assert!(stub.genome.is_empty());
        assert!(stub.epigenome.is_empty());
        assert!(stub.inventory.is_empty());
        assert!(stub.skills.is_empty());
        assert!(stub.beliefs.is_empty());
        assert!(stub.known_techs.is_empty());
        assert_eq!(stub.memory, Value::Null);
    }

    // ── prune_dead_from_groups() ───────────────────────────────────────────

    #[test]
    fn dead_ids_are_removed_from_every_group_member_ids() {
        let mut groups = vec![
            json!({ "id": "g1", "member_ids": ["a", "b", "c"] }),
            json!({ "id": "g2", "member_ids": ["c", "d"] }),
        ];
        prune_dead_from_groups(&mut groups, &["c".to_string()]);
        assert_eq!(groups[0]["member_ids"], json!(["a", "b"]));
        assert_eq!(groups[1]["member_ids"], json!(["d"]));
    }

    #[test]
    fn a_group_whose_last_member_dies_becomes_empty_not_phantom() {
        let mut groups = vec![json!({ "id": "g1", "member_ids": ["a"] })];
        prune_dead_from_groups(&mut groups, &["a".to_string()]);
        assert_eq!(groups[0]["member_ids"], json!([]), "an empty member_ids is what lets form_groups's retain() drop this group next tick");
    }

    #[test]
    fn empty_dead_id_list_is_a_no_op() {
        let mut groups = vec![json!({ "id": "g1", "member_ids": ["a", "b"] })];
        prune_dead_from_groups(&mut groups, &[]);
        assert_eq!(groups[0]["member_ids"], json!(["a", "b"]));
    }

    // ── pruneOrphanedSettlements ────────────────────────────────────────

    #[test]
    fn a_settlement_whose_group_no_longer_exists_is_dropped() {
        let groups = vec![json!({ "id": "g1", "member_ids": ["a", "b"] })];
        let mut settlements = vec![json!({ "id": "s1", "group_id": "g1" }), json!({ "id": "s2", "group_id": "g_long_dead" })];
        prune_orphaned_settlements(&mut settlements, &groups);
        assert_eq!(settlements.len(), 1);
        assert_eq!(settlements[0]["id"], "s1");
    }

    #[test]
    fn a_settlement_whose_group_still_exists_is_kept() {
        let groups = vec![json!({ "id": "g1", "member_ids": ["a", "b"] }), json!({ "id": "g2", "member_ids": ["c"] })];
        let mut settlements = vec![json!({ "id": "s1", "group_id": "g1" }), json!({ "id": "s2", "group_id": "g2" })];
        prune_orphaned_settlements(&mut settlements, &groups);
        assert_eq!(settlements.len(), 2);
    }

    #[test]
    fn no_groups_at_all_drops_every_settlement() {
        let mut settlements = vec![json!({ "id": "s1", "group_id": "g1" }), json!({ "id": "s2", "group_id": "g2" })];
        prune_orphaned_settlements(&mut settlements, &[]);
        assert!(settlements.is_empty());
    }

    #[test]
    fn a_settlement_with_no_group_id_at_all_is_dropped() {
        let groups = vec![json!({ "id": "g1", "member_ids": ["a"] })];
        let mut settlements = vec![json!({ "id": "s1" })];
        prune_orphaned_settlements(&mut settlements, &groups);
        assert!(settlements.is_empty(), "a settlement that can never resolve to a live group has no reason to be kept");
    }

    #[test]
    fn ordinary_mortality_death_prunes_the_group_within_the_same_tick() {
        // Regression test: ordinary mortality (unlike disaster deaths, which
        // environment::process_disaster already handled) used to leave a
        // dead individual's id sitting in their group's member_ids forever,
        // inflating "group size" for every downstream pass this same tick
        // (law/social/culture/settlement thresholds) and preventing
        // form_groups's retain() from ever dropping a group everyone in it
        // had actually died out of.
        let mut old = Individual {
            id: "elder".to_string(),
            alive: true,
            is_founder: true,
            group_id: Some("g1".to_string()),
            birth_day: -900 * 365, // ~900 years old: a few % daily death risk from old age alone.
            ..Default::default()
        };
        old.phenotype.max_lifespan = 70.0;
        let mut state = SimulationState {
            individuals: vec![old],
            groups: vec![json!({ "id": "g1", "member_ids": ["elder"] })],
            ..Default::default()
        };
        // roll_death is probabilistic even at extreme age -- loop until it
        // actually fires (overwhelmingly likely well within this bound) so
        // the assertion below isn't checking a specific unlucky tick.
        for _ in 0..2000 {
            advance_one_day(&mut state);
            if state.individuals[0].is_dead {
                break;
            }
        }
        assert!(state.individuals[0].is_dead, "an ~900-year-old individual should have died of old age within 2000 days");
        assert_eq!(state.groups[0]["member_ids"], json!([]), "the dead elder must not remain a phantom member of their own group");
    }

    // ── pending-pregnancy abort on maternal death (V-01 regression) ─────

    #[test]
    fn a_pregnancy_is_aborted_not_born_if_the_mother_died_before_term() {
        let mother = Individual { id: "mother".to_string(), alive: false, is_dead: true, death_day: Some(0), ..Default::default() };
        let unborn = Individual { id: "unborn".to_string(), parent_1_id: Some("mother".to_string()), birth_day: 0, ..Default::default() };
        let mut state = SimulationState { current_day: 0, individuals: vec![mother], pending_births: vec![unborn], ..Default::default() };

        advance_one_day(&mut state);

        assert!(
            state.individuals.iter().all(|i| i.id != "unborn"),
            "a child whose mother died mid-gestation must never become a living member of the population"
        );
        assert!(
            state.events.iter().any(|e| e["type"] == "pregnancy_loss" && e["individual_id"] == "unborn"),
            "a pregnancy_loss event should be recorded instead of a birth"
        );
        assert!(!state.events.iter().any(|e| e["type"] == "birth" && e["individual_id"] == "unborn"));
    }

    #[test]
    fn a_pregnancy_still_proceeds_normally_when_the_mother_is_alive() {
        let mother = Individual { id: "mother".to_string(), alive: true, ..Default::default() };
        let unborn = Individual { id: "unborn".to_string(), parent_1_id: Some("mother".to_string()), birth_day: 0, ..Default::default() };
        let mut state = SimulationState { current_day: 0, individuals: vec![mother], pending_births: vec![unborn], ..Default::default() };

        advance_one_day(&mut state);

        assert!(state.individuals.iter().any(|i| i.id == "unborn"), "a live mother's pregnancy should still result in a birth");
        assert!(state.events.iter().any(|e| e["type"] == "birth" && e["individual_id"] == "unborn"));
    }

    // ── twin/triplet epigenetics + psychology init regression ──────────────

    #[test]
    fn twin_and_triplet_siblings_get_full_epigenetics_and_psychology_init() {
        // The primary due-birth child gets epigenetics::inherit_epigenome and
        // psychology::initialize_psychology at conception time (see the
        // `conceived` loop earlier in advance_one_day); a twin/triplet
        // sibling is instead created directly inside the due-birth loop via
        // create_child(), which must get the exact same treatment or it's
        // left with an empty epigenome (silently self-healing to a flat,
        // non-heritable 0.5 at every locus the next time update_epigenome
        // runs) and an uninitialized psychology (attachment_style stuck at
        // "", never derived from oxytocin_sensitivity/anxiety).
        //
        // twin_chance is probabilistic even with fertility pinned to 1.0
        // (~5.2% per birth), so run enough independent trials that seeing at
        // least one twin is overwhelmingly likely rather than asserting on a
        // single unlucky roll.
        let mut saw_a_twin = false;
        for attempt in 0..500 {
            let mut mother = Individual { id: "mother".to_string(), alive: true, sex: "female".to_string(), ..Default::default() };
            mother.phenotype.fertility = 1.0;
            let father = Individual { id: "father".to_string(), alive: true, sex: "male".to_string(), ..Default::default() };
            let unborn = Individual {
                id: format!("unborn-{attempt}"),
                parent_1_id: Some("mother".to_string()),
                parent_2_id: Some("father".to_string()),
                birth_day: 0,
                ..Default::default()
            };
            let mut state = SimulationState { current_day: 0, individuals: vec![mother, father], pending_births: vec![unborn], ..Default::default() };

            advance_one_day(&mut state);

            for ind in &state.individuals {
                if ind.extra.get("is_twin").and_then(Value::as_bool) != Some(true) {
                    continue;
                }
                saw_a_twin = true;
                assert!(!ind.epigenome.is_empty(), "a twin/triplet's epigenome must be inherited from parents, not left empty");
                assert!(
                    ind.epigenome.values().all(|l| l.last_modified.is_some()),
                    "a twin/triplet's epigenome must come from inherit_epigenome (last_modified = Some(0)), not stay uninitialized"
                );
                assert!(
                    !ind.psychology.attachment_style.is_empty(),
                    "a twin/triplet's psychology must be initialized just like the primary child's, deriving attachment_style"
                );
            }
        }
        assert!(saw_a_twin, "expected at least one twin across 500 trials with fertility pinned to 1.0 (~5.2% chance each)");
    }

    // ── trade/disease spatial pairing regression ────────────────────────────

    #[test]
    fn trade_disease_pairing_only_pairs_individuals_within_nearby_radius() {
        // `b` is deliberately placed far away but array-adjacent to `a` (the
        // old `alive_ids.chunks(2)` pairing was purely storage-order, so it
        // would always pick (a, b) here and never reach `d`); `d` is the one
        // actually within NEARBY_RADIUS of `a` and should be the one paired.
        let mut a = Individual { id: "a".to_string(), alive: true, x: 0.0, y: 0.0, ..Default::default() };
        a.extra.insert("infections".to_string(), json!([{ "pathogen_id": "respiratory_common", "days_remaining": 9999, "infected_day": 0 }]));
        let b = Individual { id: "b".to_string(), alive: true, x: 1000.0, y: 0.0, ..Default::default() };
        let d = Individual { id: "d".to_string(), alive: true, x: 0.1, y: 0.0, ..Default::default() };

        let disabled_engines: HashSet<String> =
            ["movement", "mortality_roll", "microbiome_outbreak", "reproduction"].iter().map(|s| s.to_string()).collect();
        let mut state = SimulationState { current_day: 0, individuals: vec![a, b, d], disabled_engines, ..Default::default() };

        fn is_infected(state: &SimulationState, id: &str) -> bool {
            state.individuals.iter().find(|i| i.id == id).unwrap().extra.get("infections").and_then(Value::as_array).map(|arr| !arr.is_empty()).unwrap_or(false)
        }

        let mut d_infected = false;
        for _ in 0..150 {
            advance_one_day(&mut state);
            if is_infected(&state, "d") {
                d_infected = true;
            }
            assert!(!is_infected(&state, "b"), "an individual 1000 degrees away must never catch a's disease, even if array-adjacent to them");
        }
        assert!(d_infected, "the individual actually within NEARBY_RADIUS of the infected founder should eventually be paired and catch it");
    }

    // ── disease_pressure derived from real infection prevalence ──────────

    #[test]
    fn an_actual_outbreak_raises_world_state_disease_pressure() {
        let mut a = Individual { id: "a".to_string(), alive: true, x: 0.0, y: 0.0, ..Default::default() };
        a.extra.insert("infections".to_string(), json!([{ "pathogen_id": "respiratory_common", "days_remaining": 9999, "infected_day": 0 }]));
        let b = Individual { id: "b".to_string(), alive: true, x: 0.0, y: 0.0, ..Default::default() };
        let disabled_engines: HashSet<String> =
            ["movement", "mortality_roll", "microbiome_outbreak", "reproduction"].iter().map(|s| s.to_string()).collect();
        let mut state = SimulationState { current_day: 0, individuals: vec![a, b], disabled_engines, ..Default::default() };

        advance_one_day(&mut state);

        let disease_pressure = state.world_state.extra.get("disease_pressure").and_then(Value::as_f64).unwrap_or(0.0);
        assert!((disease_pressure - 0.5).abs() < 1e-9, "1 infected of 2 alive should yield disease_pressure 0.5, got {disease_pressure}");
    }

    #[test]
    fn a_healthy_population_has_zero_disease_pressure_not_a_flat_default() {
        let a = Individual { id: "a".to_string(), alive: true, x: 0.0, y: 0.0, ..Default::default() };
        let disabled_engines: HashSet<String> =
            ["movement", "mortality_roll", "microbiome_outbreak", "reproduction"].iter().map(|s| s.to_string()).collect();
        let mut state = SimulationState { current_day: 0, individuals: vec![a], disabled_engines, ..Default::default() };

        advance_one_day(&mut state);

        let disease_pressure = state.world_state.extra.get("disease_pressure").and_then(Value::as_f64).unwrap_or(-1.0);
        assert_eq!(disease_pressure, 0.0, "no infections should yield disease_pressure 0.0, not the old flat 0.1 default");
    }

    // ── are_kin() ────────────────────────────────────────────────────────

    #[test]
    fn parent_and_child_are_kin_in_either_direction() {
        let parent = Individual { id: "p".to_string(), ..Default::default() };
        let child = Individual { id: "c".to_string(), parent_1_id: Some("p".to_string()), ..Default::default() };
        assert!(are_kin(&parent, &child));
        assert!(are_kin(&child, &parent));
    }

    #[test]
    fn shared_parent_makes_siblings_kin() {
        let a = Individual { id: "a".to_string(), parent_1_id: Some("p1".to_string()), parent_2_id: Some("p2".to_string()), ..Default::default() };
        let b = Individual { id: "b".to_string(), parent_1_id: Some("p2".to_string()), parent_2_id: Some("p3".to_string()), ..Default::default() };
        assert!(are_kin(&a, &b), "sharing one bio-parent (p2) makes them at least half-siblings");
    }

    #[test]
    fn unrelated_individuals_are_not_kin() {
        let a = Individual { id: "a".to_string(), parent_1_id: Some("p1".to_string()), ..Default::default() };
        let b = Individual { id: "b".to_string(), parent_1_id: Some("p2".to_string()), ..Default::default() };
        assert!(!are_kin(&a, &b));
    }

    // ── apply_death_witnessing() ─────────────────────────────────────────

    fn make_survivor(id: &str, x: f64, y: f64) -> Individual {
        Individual { id: id.to_string(), alive: true, x, y, ..Default::default() }
    }

    fn make_deceased(id: &str, x: f64, y: f64, cause: &str, death_day: i32) -> Individual {
        let mut ind = Individual { id: id.to_string(), alive: false, is_dead: true, x, y, death_day: Some(death_day), ..Default::default() };
        ind.extra.insert("death_cause".to_string(), json!(cause));
        ind
    }

    #[test]
    fn kin_witness_gets_a_stronger_water_fear_bump_than_a_bystander() {
        let mut state = crate::state::SimulationState::default();
        let mut child_of_deceased = make_survivor("child", 0.1, 0.0);
        child_of_deceased.parent_1_id = Some("dead".to_string());
        let bystander = make_survivor("bystander", 0.1, 0.0);
        let dead = make_deceased("dead", 0.0, 0.0, "drowning", 9);

        state.individuals = vec![child_of_deceased, bystander, dead];
        state.current_day = 10; // yesterday == 9, matches the deceased's death_day

        let kin_events = apply_death_witnessing(&mut state, 10);

        let child_fear = state.individuals[0].extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
        let bystander_fear = state.individuals[1].extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(child_fear > bystander_fear, "kin (0.7 weight) should react more strongly than a bystander (0.4 weight)");
        assert!(bystander_fear > 0.0, "a nearby bystander should still pick up some water fear");
        assert!(kin_events.iter().any(|e| e["individual_id"] == "child"), "the child should get a death_of_kin event");
        assert!(!kin_events.iter().any(|e| e["individual_id"] == "bystander"), "a non-kin bystander should not get a death_of_kin event");
    }

    #[test]
    fn witnessing_is_bounded_by_witness_radius() {
        let mut state = crate::state::SimulationState::default();
        let far_witness = make_survivor("far", 100.0, 100.0);
        let dead = make_deceased("dead", 0.0, 0.0, "predator", 4);
        state.individuals = vec![far_witness, dead];

        apply_death_witnessing(&mut state, 5);

        assert!(state.individuals[0].extra.get("_fears").is_none(), "someone far outside WITNESS_RADIUS should be unaffected");
    }

    #[test]
    fn only_yesterdays_deaths_are_witnessed() {
        let mut state = crate::state::SimulationState::default();
        let nearby = make_survivor("nearby", 0.0, 0.0);
        let dead = make_deceased("dead", 0.0, 0.0, "predator", 3); // died on day 3
        state.individuals = vec![nearby, dead];

        apply_death_witnessing(&mut state, 10); // "today" is day 10, not day 4

        assert!(state.individuals[0].extra.get("_fears").is_none(), "a death from several days ago should not trigger fresh witnessing");
    }

    #[test]
    fn non_water_death_cause_raises_the_matching_fears_key() {
        let mut state = crate::state::SimulationState::default();
        let witness = make_survivor("witness", 0.0, 0.0);
        let dead = make_deceased("dead", 0.05, 0.0, "predator", 5);
        state.individuals = vec![witness, dead];

        apply_death_witnessing(&mut state, 6);

        let predator_fear = state.individuals[0].extra["_fears"]["predator"].as_f64().unwrap_or(0.0);
        assert!(predator_fear > 0.0);
    }

    #[test]
    fn a_wildlife_encounter_death_also_raises_predator_fear() {
        let mut state = crate::state::SimulationState::default();
        let witness = make_survivor("witness", 0.0, 0.0);
        let dead = make_deceased("dead", 0.05, 0.0, "wildlife_encounter", 5);
        state.individuals = vec![witness, dead];

        apply_death_witnessing(&mut state, 6);

        let predator_fear = state.individuals[0].extra["_fears"]["predator"].as_f64().unwrap_or(0.0);
        assert!(predator_fear > 0.0);
    }

    #[test]
    fn starvation_and_dehydration_deaths_raise_scarcity_fear_not_general() {
        for cause in ["starvation", "dehydration"] {
            let mut state = crate::state::SimulationState::default();
            let witness = make_survivor("witness", 0.0, 0.0);
            let dead = make_deceased("dead", 0.05, 0.0, cause, 5);
            state.individuals = vec![witness, dead];

            apply_death_witnessing(&mut state, 6);

            let fears = &state.individuals[0].extra["_fears"];
            assert!(fears.get("scarcity").and_then(Value::as_f64).unwrap_or(0.0) > 0.0, "{cause} should raise scarcity fear");
            assert!(fears.get("general").is_none(), "{cause} should not fall back to the general bucket");
        }
    }

    #[test]
    fn disaster_type_deaths_raise_disaster_fear_not_general() {
        for cause in ["earthquake", "flood", "wildfire", "blizzard_disaster", "drought_event"] {
            let mut state = crate::state::SimulationState::default();
            let witness = make_survivor("witness", 0.0, 0.0);
            let dead = make_deceased("dead", 0.05, 0.0, cause, 5);
            state.individuals = vec![witness, dead];

            apply_death_witnessing(&mut state, 6);

            let fears = &state.individuals[0].extra["_fears"];
            assert!(fears.get("disaster").and_then(Value::as_f64).unwrap_or(0.0) > 0.0, "{cause} should raise disaster fear");
            assert!(fears.get("general").is_none(), "{cause} should not fall back to the general bucket");
        }
    }

    #[test]
    fn an_unmapped_cause_still_falls_back_to_general() {
        let mut state = crate::state::SimulationState::default();
        let witness = make_survivor("witness", 0.0, 0.0);
        let dead = make_deceased("dead", 0.05, 0.0, "genetic_disease", 5);
        state.individuals = vec![witness, dead];

        apply_death_witnessing(&mut state, 6);

        let general_fear = state.individuals[0].extra["_fears"]["general"].as_f64().unwrap_or(0.0);
        assert!(general_fear > 0.0);
    }

    // ── update_water_state() ─────────────────────────────────────────────

    #[test]
    fn in_water_countdown_grants_water_experience_on_expiry_and_then_clears() {
        let mut ind = Individual::default();
        ind.extra.insert("_inWaterDaysRemaining".to_string(), json!(1));
        environment::update_water_state(&mut ind);
        assert_eq!(ind.extra.get("_inWater").and_then(Value::as_bool), Some(true));
        assert!(ind.extra.get("_waterExperience").and_then(Value::as_f64).unwrap_or(0.0) > 0.0, "surviving the last day in water should grant experience");

        environment::update_water_state(&mut ind);
        assert_eq!(ind.extra.get("_inWater").and_then(Value::as_bool), Some(false), "should be back on dry land once the countdown is fully spent");
    }

    #[test]
    fn water_fear_decays_toward_zero_each_tick() {
        let mut ind = Individual::default();
        ind.extra.insert("_waterFear".to_string(), json!(0.1));
        environment::update_water_state(&mut ind);
        let fear = ind.extra.get("_waterFear").and_then(Value::as_f64).unwrap();
        assert!(fear < 0.1 && fear > 0.0);
    }

    // ── juvenile family cohesion ──────────────────────────────────────────

    /// Builds a scenario where the persisted wander heading points directly
    /// away from the parent (angle = PI, parent at +x). "explore"'s jitter is
    /// bounded to ±0.6 rad around that heading, which keeps its raw cos()
    /// negative for every possible roll -- so any *positive* net x movement
    /// can only come from the juvenile pull, making this deterministic
    /// despite the RNG in the explore jitter and persisted-angle fallback.
    /// Coordinates are deep in the Sahara (well clear of any coastline within
    /// DAILY_STEP's range) since is_on_land() now gates movement on a real
    /// land/water raster instead of a coarse bounding box.
    fn away_from_parent_scenario(age_days: i32) -> (Individual, Individual) {
        let parent = Individual { id: "parent".to_string(), alive: true, x: 15.0, y: 25.0, group_id: Some("g".to_string()), ..Default::default() };
        let mut child = Individual {
            id: "child".to_string(),
            alive: true,
            x: 10.0,
            y: 25.0,
            age_days: Some(age_days),
            parent_1_id: Some("parent".to_string()),
            group_id: Some("g".to_string()),
            ..Default::default()
        };
        child.extra.insert("_currentAction".to_string(), json!("explore"));
        child.extra.insert("_moveAngle".to_string(), json!(std::f64::consts::PI));
        (parent, child)
    }

    #[test]
    fn juvenile_movement_is_pulled_toward_a_living_parent() {
        let mut state = crate::state::SimulationState::default();
        let (parent, child) = away_from_parent_scenario(365 * 3); // 3 years old
        let snapshot = vec![parent.clone(), child.clone()];
        let positions: Vec<(f64, f64)> = snapshot.iter().map(|i| (i.x, i.y)).collect();
        let grid = SpatialGrid::build(&positions, NEARBY_RADIUS);

        state.individuals = vec![parent, child];
        apply_movement(&mut state, &snapshot, &grid, 1, 0.0, 1.0);

        assert!(state.individuals[1].x > 10.0, "a 3-year-old's parent-pull should overpower a wander heading pointed straight away from the parent");
    }

    #[test]
    fn adult_movement_is_not_pulled_toward_a_parent() {
        let mut state = crate::state::SimulationState::default();
        let (parent, adult_child) = away_from_parent_scenario(365 * 20); // 20 years old, well past JUVENILE_MAX_AGE_YEARS
        let snapshot = vec![parent.clone(), adult_child.clone()];
        let positions: Vec<(f64, f64)> = snapshot.iter().map(|i| (i.x, i.y)).collect();
        let grid = SpatialGrid::build(&positions, NEARBY_RADIUS);

        state.individuals = vec![parent, adult_child];
        apply_movement(&mut state, &snapshot, &grid, 1, 0.0, 1.0);

        assert!(state.individuals[1].x < 10.0, "an adult should freely follow the wander heading away from the parent, unaffected by the juvenile pull");
    }

    // ── disabled_engines (diagnostic per-engine toggles) ────────────────

    /// Deep in the Sahara -- see away_from_parent_scenario's own comment on
    /// why: well clear of any coastline within a day's movement, so
    /// is_on_land() never blocks the move this test cares about isolating.
    fn on_land_adult() -> Individual {
        Individual {
            id: "a".to_string(),
            alive: true,
            sex: "female".to_string(),
            birth_day: -365 * 25,
            x: 10.0,
            y: 25.0,
            health: crate::types::Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
            ..Default::default()
        }
    }

    /// Everything except `movement` disabled, with `_currentAction` forced to
    /// "explore" -- otherwise microbiome_agent's own select_action call (which
    /// runs before movement) could rationally pick "rest" for a fully-satisfied
    /// individual, and apply_movement skips "rest"/"craft" regardless of any
    /// toggle. Disabling microbiome_agent here means nothing overwrites the
    /// forced action before movement reads it.
    fn movement_only_scenario() -> crate::state::SimulationState {
        let mut ind = on_land_adult();
        ind.extra.insert("_currentAction".to_string(), json!("explore"));
        let disabled_engines: HashSet<String> = crate::state::TOGGLEABLE_ENGINES
            .iter()
            .filter(|&&name| name != "movement")
            .map(|s| s.to_string())
            .collect();
        crate::state::SimulationState {
            current_day: 100,
            individuals: vec![ind],
            disabled_engines,
            ..Default::default()
        }
    }

    #[test]
    fn disabling_the_movement_engine_leaves_positions_untouched() {
        let mut state = movement_only_scenario();
        state.disabled_engines.insert("movement".to_string());
        let before = (state.individuals[0].x, state.individuals[0].y);
        advance_one_day(&mut state);
        assert_eq!((state.individuals[0].x, state.individuals[0].y), before, "disabling the movement engine should leave positions untouched");
    }

    #[test]
    fn movement_runs_when_not_disabled() {
        let mut state = movement_only_scenario();
        let before = (state.individuals[0].x, state.individuals[0].y);
        advance_one_day(&mut state);
        assert_ne!((state.individuals[0].x, state.individuals[0].y), before, "with movement enabled (and forced to \"explore\"), this on-land adult should have moved");
    }
}
