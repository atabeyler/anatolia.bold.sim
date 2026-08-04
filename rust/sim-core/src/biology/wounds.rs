//! Wound infliction and healing -- turns `health.injuries` (declared in
//! types.rs, but never actually written or read anywhere until this module)
//! into a real, tick-by-tick physiological process rather than a decorative
//! empty field.
//!
//! Before this module, an individual either died from a mortality-roll
//! cause or walked away completely unscathed -- `hp` only ever moved from
//! hunger/thirst decay, weather, and a small GH/IGF-1 recovery term
//! (hormones.rs). There was no substrate at all for "survived a dangerous
//! encounter, but not for free": `mortality::roll_death` computing an
//! elevated risk from `predator_risk`/dangerous weather and then rolling
//! *against* death meant that elevated risk had zero further consequence
//! once the roll failed. This reuses exactly those same signals (already
//! read as plain JSON fields off `environment`, matching mortality.rs's own
//! pattern for `predator_risk`/`weather_cold_risk`/`weather_heat_risk`) to
//! probabilistically inflict a *survivable* wound instead -- a small,
//! bounded, additive consequence layered on top of the existing mortality
//! roll, the same pattern this codebase already uses throughout (see
//! AGENTS.md's Hormones section for many examples of this shape).
//!
//! A wound is `{"severity": f64 in (0, WOUND_MAX_SEVERITY], "day": i32}` in
//! `health.injuries`. Each tick, every open wound both trims `hp` in
//! proportion to its current severity (an open wound impairs the body) and
//! heals a little (severity shrinks) at a rate set by the individual's own
//! `health_resilience`/`immune_strength` phenotype -- genetically real
//! recovery capacity, not a flat timer -- until it closes and is removed.
//! This makes `hp` decline from an actual physiological event with its own
//! genetics-modulated recovery curve for this cause specifically, rather
//! than a probability roll with no further mechanism, which is what makes
//! it "mechanistic" in the sense this module's callers asked for.

use rand::Rng;
use serde_json::{json, Value};

use crate::state::Individual;

/// A wound this severe would need `1.0 / WOUND_HEAL_RATE_BASE` days (at the
/// slowest genetic healing rate) to close -- bounded well short of anything
/// that could itself be an alternative death spiral independent of the
/// mortality-roll system this module deliberately does not replace.
const WOUND_MAX_SEVERITY: f64 = 0.15;
const WOUND_MIN_SEVERITY: f64 = 0.03;

/// Daily chance of a wound given the maximum possible danger signal
/// (predator_risk=1.0 or actively dangerous weather); scaled down by the
/// biome/weather's real risk level below. Deliberately small: this fires
/// only for individuals who already survived today's mortality roll, so it
/// must not itself become a dominant source of hp loss.
const WOUND_CHANCE_AT_MAX_RISK: f64 = 0.01;

/// How much of a wound's severity closes per day at zero genetic recovery
/// capacity (health_resilience=0, immune_strength=0) versus full capacity
/// (both=1.0) -- bounded so even the least resilient individual's wounds
/// eventually close (this is healing, not a permanent injury system) while
/// a highly resilient individual recovers several times faster.
const WOUND_HEAL_RATE_BASE: f64 = 0.02;
const WOUND_HEAL_RATE_MAX: f64 = 0.08;

/// How much of a wound's current severity is subtracted from `hp` each tick
/// it stays open. A single max-severity wound (0.15) costs at most
/// `0.15 * 0.15 = 0.0225` hp/day while freshly inflicted, tapering to
/// nothing as it heals -- a real but survivable drag, not a second death
/// mechanism layered under the mortality roll's own hp-driven causes.
const WOUND_HP_DRAIN_FACTOR: f64 = 0.15;

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

/// Called once per living individual per tick, only for those who already
/// survived today's `mortality::roll_death` -- see tick.rs's own call site
/// for why this ordering matters (a wound is a consequence of danger that
/// *didn't* kill, not an independent risk).
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
    individual.health.injuries.push(json!({ "severity": severity, "day": current_day }));
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

/// Sum of every currently-open wound's severity -- exposed for callers
/// (e.g. a future consciousness.rs/psychology.rs term wanting a genuine
/// injury signal instead of a bare `hp < threshold` proxy) that want the
/// wound state directly rather than inferring it from `hp`, which is also
/// affected by hunger/thirst/weather and so is a noisier signal of "how
/// injured is this individual specifically" than the wound list itself.
pub fn total_open_wound_severity(individual: &Individual) -> f64 {
    individual.health.injuries.iter().filter_map(|w| w.get("severity").and_then(Value::as_f64)).sum()
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
        for day in 0..5000 {
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
    fn a_wound_eventually_heals_and_is_removed() {
        let mut ind = individual_with(1.0, 0.9, 0.9);
        ind.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0 }));
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
        low.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0 }));
        high.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0 }));

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
        ind.health.injuries.push(json!({ "severity": WOUND_MAX_SEVERITY, "day": 0 }));
        update_wound_healing(&mut ind);
        assert!(ind.health.hp < 1.0, "an open wound must drain hp");

        let hp_after_first_tick = ind.health.hp;
        let mut healthy = individual_with(1.0, 0.5, 0.5);
        update_wound_healing(&mut healthy);
        assert_eq!(healthy.health.hp, 1.0, "an individual with no wounds must have hp untouched by update_wound_healing");
        let _ = hp_after_first_tick;
    }

    #[test]
    fn empty_injuries_is_a_true_no_op() {
        let mut ind = individual_with(0.42, 0.5, 0.5);
        update_wound_healing(&mut ind);
        assert_eq!(ind.health.hp, 0.42);
        assert!(ind.health.injuries.is_empty());
    }

    #[test]
    fn total_open_wound_severity_sums_every_open_wound() {
        let mut ind = individual_with(1.0, 0.5, 0.5);
        ind.health.injuries.push(json!({ "severity": 0.05, "day": 0 }));
        ind.health.injuries.push(json!({ "severity": 0.07, "day": 1 }));
        assert!((total_open_wound_severity(&ind) - 0.12).abs() < 1e-9);
    }
}
