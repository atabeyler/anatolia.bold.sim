//! Wound infliction, healing, and wound-driven death -- turns
//! `health.injuries` (declared in types.rs, but never actually read or
//! written anywhere until this module) into a real, tick-by-tick
//! physiological process, and makes Predator/Injury/WildlifeEncounter/
//! Exposure emerge from that process instead of from a probability roll.
//!
//! Before this module, an individual either died from a `mortality::roll_death`
//! cause picked by `determine_cause`'s own internal probability cascade --
//! including Predator/Injury/WildlifeEncounter/Exposure, resolved as a
//! narrative label for a death that roll had *already* decided was
//! happening, with no requirement the individual had sustained any physical
//! harm first -- or walked away from a dangerous encounter completely
//! unscathed. There was no substrate at all for "survived a dangerous
//! encounter, but not for free," and no way for those four causes to be
//! anything but a coin flip layered under an unrelated daily death roll.
//!
//! This module is now the *sole* source of those four causes:
//! `maybe_inflict_wound` reuses exactly the same signals `mortality.rs`
//! used to gate them (`predator_risk`, `weather_cold_risk`/`weather_heat_risk`)
//! to probabilistically wound a survivor of today's (unrelated,
//! non-wound-related) death roll, tagging each wound with the circumstance
//! that caused it. Every open wound drains `hp` in proportion to its
//! current severity each tick and heals at a rate set by the individual's
//! own `health_resilience`/`immune_strength` phenotype -- real genetic
//! recovery capacity, not a flat timer. A single wound is deliberately
//! survivable on its own (bounded severity, a healing rate that always
//! eventually closes it), but wounds accumulate if inflicted faster than
//! they heal, and if their combined severity drives `hp` to 0,
//! `wound_collapse_cause` reports which of the four causes applies --
//! whichever open wound is currently most severe, using the circumstance
//! it was tagged with at infliction. This is death as the natural
//! consequence of a real physiological state crossing zero, not a
//! probability roll independent of that state.
//!
//! `mortality::compute_daily_death_risk`'s own base_risk is reduced
//! (`NON_WOUND_CAUSE_SHARE`) to hand these four causes' prior share of
//! overall mortality to this mechanism instead of simply deleting it --
//! see that constant's own doc comment for how the reduction was estimated,
//! and `tests/empirical_validation.rs` for the harness that validates the
//! resulting *combined* (roll_death + wound-collapse) mortality rate
//! empirically rather than trusting either estimate exactly.

use rand::Rng;
use serde_json::{json, Value};

use super::mortality::{resolve_misadventure, DeathCause};
use crate::state::Individual;

/// A wound this severe would need `1.0 / WOUND_HEAL_RATE_BASE` days (at the
/// slowest genetic healing rate) to close on its own -- survivable in
/// isolation. Multiple simultaneous wounds (inflicted faster than they
/// heal) are what make sustained danger exposure actually lethal -- see
/// this module's own doc comment.
const WOUND_MAX_SEVERITY: f64 = 0.15;
const WOUND_MIN_SEVERITY: f64 = 0.03;

/// Daily chance of a wound given the maximum possible danger signal
/// (predator_risk=1.0 or actively dangerous weather); scaled down by the
/// biome/weather's real risk level below. This fires only for individuals
/// who already survived today's unrelated mortality roll, and needs to be
/// high enough that sustained high-danger exposure can realistically
/// out-pace healing and become lethal, without dominating every other
/// cause of death. `empirical_validation.rs`'s Monte Carlo harness measured
/// this mechanism's actual contribution directly (per-cause death tallies)
/// and found it produces close to zero deaths at this value across a
/// 15-replicate/15-year run -- the age-band overshoot that harness first
/// caught (5-15y at ~3-6x its documented target) turned out to come almost
/// entirely from `compute_daily_death_risk`'s own chronic-cortisol term
/// (mortality.rs), not from this mechanism; see that file's own note.
/// This value is a reasonable middle ground pending a real calibration pass
/// once that separate, pre-existing psychology/hormones-stress issue is
/// fixed and this mechanism's own share of total mortality can be measured
/// cleanly again.
const WOUND_CHANCE_AT_MAX_RISK: f64 = 0.02;

/// How much of a wound's severity closes per day at zero genetic recovery
/// capacity (health_resilience=0, immune_strength=0) versus full capacity
/// (both=1.0) -- bounded so even the least resilient individual's *isolated*
/// wound eventually closes, while a highly resilient individual recovers
/// several times faster and can shrug off sustained danger that would kill
/// a frailer one.
const WOUND_HEAL_RATE_BASE: f64 = 0.015;
const WOUND_HEAL_RATE_MAX: f64 = 0.08;

/// How much of a wound's current severity is subtracted from `hp` each tick
/// it stays open. A single max-severity wound (0.15) costs at most
/// `0.15 * 0.15 = 0.0225` hp/day while freshly inflicted, tapering to
/// nothing as it heals -- survivable alone; several simultaneous wounds
/// compound linearly, which is what can drive hp to 0.
const WOUND_HP_DRAIN_FACTOR: f64 = 0.15;

/// Which real-world circumstance a wound was inflicted under, captured at
/// the moment of infliction (not re-derived later, since the individual's
/// current environment may have changed by the time the wound proves
/// fatal) -- maps directly onto the four `DeathCause` variants this module
/// is the sole source of. Mirrors the exact same signals/thresholds
/// `mortality.rs`'s `determine_cause`/`resolve_misadventure` used to
/// resolve these from, just captured earlier (at infliction) rather than
/// later (at an unrelated death roll).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WoundOrigin {
    Predator,
    Wildlife,
    Exposure,
    Injury,
}

impl WoundOrigin {
    fn as_str(self) -> &'static str {
        match self {
            WoundOrigin::Predator => "predator",
            WoundOrigin::Wildlife => "wildlife",
            WoundOrigin::Exposure => "exposure",
            WoundOrigin::Injury => "injury",
        }
    }

    fn from_str(s: &str) -> WoundOrigin {
        match s {
            "predator" => WoundOrigin::Predator,
            "wildlife" => WoundOrigin::Wildlife,
            "exposure" => WoundOrigin::Exposure,
            _ => WoundOrigin::Injury,
        }
    }

    fn to_death_cause(self) -> DeathCause {
        match self {
            WoundOrigin::Predator => DeathCause::Predator,
            WoundOrigin::Wildlife => DeathCause::WildlifeEncounter,
            WoundOrigin::Exposure => DeathCause::Exposure,
            WoundOrigin::Injury => DeathCause::Injury,
        }
    }
}

fn danger_signal(environment: Option<&Value>) -> f64 {
    let Some(env) = environment else { return 0.0 };
    let predator_risk = env.get("predator_risk").and_then(Value::as_f64).unwrap_or(0.0);
    let cold_risk = env.get("weather_cold_risk").and_then(Value::as_bool).unwrap_or(false);
    let heat_risk = env.get("weather_heat_risk").and_then(Value::as_bool).unwrap_or(false);
    let weather_danger = if cold_risk || heat_risk { 1.0 } else { 0.0 };
    // The two signals aren't mutually exclusive (a storm in a predator-rich
    // biome is worse than either alone), but must stay bounded at 1.0 -- the
    // same clamp mortality.rs's own predator_risk/weather terms use.
    (predator_risk + weather_danger * 0.5).min(1.0)
}

/// Resolves what circumstance a freshly-inflicted wound should be tagged
/// with, from the same signals/thresholds `mortality.rs` used to resolve
/// these four causes directly. Weather danger takes priority (matching
/// `resolve_misadventure`'s own ordering), then a real predator_risk-scaled
/// chance of the dedicated large-carnivore case (matching the old
/// `predator_threshold` gate) or the smaller wildlife-encounter case
/// (matching `resolve_misadventure`'s own wildlife roll), else the residual
/// physical-mishap case.
fn resolve_wound_origin(is_founder: bool, environment: Option<&Value>) -> WoundOrigin {
    match resolve_misadventure(environment) {
        DeathCause::Exposure => return WoundOrigin::Exposure,
        DeathCause::WildlifeEncounter => return WoundOrigin::Wildlife,
        _ => {}
    }
    let predator_risk = environment.and_then(|env| env.get("predator_risk")).and_then(Value::as_f64).unwrap_or(0.0);
    let predator_threshold = if is_founder { 0.15 } else { 0.3 };
    if predator_risk > 0.35 && rand::thread_rng().gen::<f64>() < predator_threshold {
        return WoundOrigin::Predator;
    }
    WoundOrigin::Injury
}

/// Called once per living individual per tick, only for those who already
/// survived today's `mortality::roll_death` -- see tick.rs's own call site
/// for why this ordering matters (a wound is a consequence of danger that
/// *didn't* kill via that unrelated roll, not an independent risk on top
/// of it).
pub fn maybe_inflict_wound(individual: &mut Individual, current_day: i32, environment: Option<&Value>) {
    let risk = danger_signal(environment);
    if risk <= 0.0 {
        return;
    }
    let chance = risk * WOUND_CHANCE_AT_MAX_RISK;
    if rand::thread_rng().gen::<f64>() >= chance {
        return;
    }
    // Severity scales with the same risk signal that triggered it (a storm
    // in a high-predator biome inflicts a worse wound than a mild one),
    // plus a little randomness so not every wound at a given risk level is
    // identical.
    let severity = (WOUND_MIN_SEVERITY + (WOUND_MAX_SEVERITY - WOUND_MIN_SEVERITY) * risk * rand::thread_rng().gen::<f64>()).clamp(WOUND_MIN_SEVERITY, WOUND_MAX_SEVERITY);
    let origin = resolve_wound_origin(individual.is_founder, environment);
    individual.health.injuries.push(json!({ "severity": severity, "day": current_day, "origin": origin.as_str() }));
}

/// Called once per living individual per tick, unconditionally (independent
/// of whether `maybe_inflict_wound` fired this tick) -- ages every open
/// wound toward closure and applies its current hp drain. Individuals with
/// no open wounds pay no cost and do no work beyond the empty-Vec check.
pub fn update_wound_healing(individual: &mut Individual) {
    if individual.health.injuries.is_empty() {
        return;
    }
    let resilience = individual.phenotype.health_resilience.clamp(0.0, 1.0);
    let immune = individual.phenotype.immune_strength.clamp(0.0, 1.0);
    let heal_rate = WOUND_HEAL_RATE_BASE + (WOUND_HEAL_RATE_MAX - WOUND_HEAL_RATE_BASE) * ((resilience + immune) / 2.0);

    let mut total_drain = 0.0;
    let mut still_open = Vec::with_capacity(individual.health.injuries.len());
    for wound in individual.health.injuries.drain(..) {
        let severity = wound.get("severity").and_then(Value::as_f64).unwrap_or(0.0);
        if severity <= 0.0 {
            continue;
        }
        total_drain += severity * WOUND_HP_DRAIN_FACTOR;
        let healed = (severity - heal_rate).max(0.0);
        if healed > 0.0 {
            let mut updated = wound;
            updated["severity"] = json!(healed);
            still_open.push(updated);
        }
    }
    individual.health.injuries = still_open;
    if total_drain > 0.0 {
        individual.health.hp = (individual.health.hp - total_drain).max(0.0);
    }
}

/// The sole resolver of Predator/Injury/WildlifeEncounter/Exposure deaths.
/// Returns `Some(cause)` only when the individual's `hp` has actually been
/// driven to 0 *and* they currently carry at least one open wound (so a
/// starvation/dehydration/weather-hp-drain death with no wounds at all
/// isn't mistakenly attributed here -- those remain `mortality.rs`'s own
/// causes). The cause is taken from whichever open wound is currently most
/// severe, on the reasoning that the most severe untreated wound is the
/// most plausible proximate cause of a multi-wound collapse.
pub fn wound_collapse_cause(individual: &Individual) -> Option<DeathCause> {
    if individual.health.hp > 0.0 || individual.health.injuries.is_empty() {
        return None;
    }
    let worst = individual
        .health
        .injuries
        .iter()
        .max_by(|a, b| {
            let sa = a.get("severity").and_then(Value::as_f64).unwrap_or(0.0);
            let sb = b.get("severity").and_then(Value::as_f64).unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })?;
    let origin = worst.get("origin").and_then(Value::as_str).map(WoundOrigin::from_str).unwrap_or(WoundOrigin::Injury);
    Some(origin.to_death_cause())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype};

    fn individual_with(hp: f64, resilience: f64, immune: f64) -> Individual {
        Individual {
            health: Health { hp, ..Default::default() },
            phenotype: Phenotype { health_resilience: resilience, immune_strength: immune, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn no_danger_signal_never_inflicts_a_wound() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        for day in 0..1000 {
            maybe_inflict_wound(&mut ind, day, Some(&json!({ "predator_risk": 0.0 })));
        }
        assert!(ind.health.injuries.is_empty(), "zero danger signal must never inflict a wound");
    }

    #[test]
    fn no_environment_never_inflicts_a_wound() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        for day in 0..1000 {
            maybe_inflict_wound(&mut ind, day, None);
        }
        assert!(ind.health.injuries.is_empty());
    }

    #[test]
    fn high_danger_signal_eventually_inflicts_a_wound() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        let env = json!({ "predator_risk": 1.0 });
        let mut wounded = false;
        for day in 0..2000 {
            maybe_inflict_wound(&mut ind, day, Some(&env));
            if !ind.health.injuries.is_empty() {
                wounded = true;
                break;
            }
        }
        assert!(wounded, "sustained maximum danger signal should eventually inflict a wound");
    }

    #[test]
    fn wound_severity_is_bounded() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        let env = json!({ "predator_risk": 1.0 });
        for day in 0..20000 {
            maybe_inflict_wound(&mut ind, day, Some(&env));
        }
        for wound in &ind.health.injuries {
            let severity = wound.get("severity").and_then(Value::as_f64).unwrap();
            assert!(severity <= WOUND_MAX_SEVERITY + 1e-9, "wound severity {severity} exceeded WOUND_MAX_SEVERITY");
            assert!(severity >= WOUND_MIN_SEVERITY - 1e-9, "wound severity {severity} below WOUND_MIN_SEVERITY");
        }
    }

    #[test]
    fn every_wound_is_tagged_with_a_valid_origin() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        let env = json!({ "predator_risk": 1.0 });
        for day in 0..20000 {
            maybe_inflict_wound(&mut ind, day, Some(&env));
        }
        assert!(!ind.health.injuries.is_empty());
        for wound in &ind.health.injuries {
            let origin = wound.get("origin").and_then(Value::as_str).expect("every wound must carry an origin tag");
            assert!(["predator", "wildlife", "exposure", "injury"].contains(&origin), "unexpected wound origin tag: {origin}");
        }
    }

    #[test]
    fn a_wound_eventually_heals_and_is_removed() {
        let mut ind = individual_with(1.0, 0.9, 0.9);
        ind.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0, "origin": "injury" }));
        for _ in 0..1000 {
            update_wound_healing(&mut ind);
            if ind.health.injuries.is_empty() {
                break;
            }
        }
        assert!(ind.health.injuries.is_empty(), "a wound must eventually fully heal and be removed from health.injuries");
    }

    #[test]
    fn higher_resilience_and_immunity_heal_strictly_faster() {
        let mut low = individual_with(1.0, 0.0, 0.0);
        let mut high = individual_with(1.0, 1.0, 1.0);
        low.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0, "origin": "injury" }));
        high.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0, "origin": "injury" }));

        let mut low_days = None;
        let mut high_days = None;
        for day in 0..2000 {
            update_wound_healing(&mut low);
            update_wound_healing(&mut high);
            if low_days.is_none() && low.health.injuries.is_empty() {
                low_days = Some(day);
            }
            if high_days.is_none() && high.health.injuries.is_empty() {
                high_days = Some(day);
            }
        }
        let low_days = low_days.expect("low-resilience wound must still fully heal");
        let high_days = high_days.expect("high-resilience wound must still fully heal");
        assert!(high_days < low_days, "higher resilience/immunity ({high_days} days) must heal strictly faster than lower ({low_days} days)");
    }

    #[test]
    fn an_open_wound_drains_hp_while_a_healed_one_does_not() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        ind.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0, "origin": "injury" }));
        update_wound_healing(&mut ind);
        assert!(ind.health.hp < 1.0, "an open wound must drain hp");

        let mut healthy = individual_with(1.0, 0.5, 0.5);
        update_wound_healing(&mut healthy);
        assert_eq!(healthy.health.hp, 1.0, "an individual with no wounds must have hp untouched by update_wound_healing");
    }

    #[test]
    fn empty_injuries_is_a_true_no_op() {
        let mut ind = individual_with(0.42, 0.5, 0.5);
        update_wound_healing(&mut ind);
        assert_eq!(ind.health.hp, 0.42);
        assert!(ind.health.injuries.is_empty());
    }

    #[test]
    fn sustained_wounding_faster_than_healing_can_drive_hp_to_zero() {
        // The actual "wounds can kill" property this module exists for:
        // a low-resilience individual repeatedly wounded under sustained
        // maximum danger, faster than their own slow healing can keep up,
        // must eventually reach hp=0 -- not merely lose a bounded, always-
        // survivable amount.
        let mut ind = individual_with(1.0, 0.0, 0.0);
        let env = json!({ "predator_risk": 1.0 });
        let mut collapsed = false;
        for day in 0..50_000 {
            maybe_inflict_wound(&mut ind, day, Some(&env));
            update_wound_healing(&mut ind);
            if ind.health.hp <= 0.0 {
                collapsed = true;
                break;
            }
        }
        assert!(collapsed, "sustained maximum danger with zero genetic resilience should eventually drive hp to 0 via accumulated wounds");
    }

    #[test]
    fn wound_collapse_cause_is_none_without_open_wounds_even_at_zero_hp() {
        let ind = individual_with(0.0, 0.5, 0.5);
        assert_eq!(wound_collapse_cause(&ind), None, "hp=0 with no wounds must not be attributed to this module -- that's mortality.rs's own domain (e.g. starvation)");
    }

    #[test]
    fn wound_collapse_cause_is_none_above_zero_hp_even_with_open_wounds() {
        let mut ind = individual_with(0.5, 0.5, 0.5);
        ind.health.injuries.push(json!({ "severity": 0.1, "day": 0, "origin": "predator" }));
        assert_eq!(wound_collapse_cause(&ind), None);
    }

    #[test]
    fn wound_collapse_cause_matches_the_most_severe_open_wound_origin() {
        let mut ind = individual_with(0.0, 0.5, 0.5);
        ind.health.injuries.push(json!({ "severity": 0.02, "day": 0, "origin": "exposure" }));
        ind.health.injuries.push(json!({ "severity": 0.12, "day": 1, "origin": "predator" }));
        ind.health.injuries.push(json!({ "severity": 0.05, "day": 2, "origin": "injury" }));
        assert_eq!(wound_collapse_cause(&ind), Some(DeathCause::Predator));
    }
}
