use std::collections::HashSet;

use serde_json::{json, Value};

#[allow(clippy::type_complexity)]
pub const ASTRONOMY_KNOWLEDGE: &[(&str, &[&str], f64, f64, &[&str])] = &[
    ("lunar_tracking", &["lunar_cycle"], 0.3, 0.4, &[]),
    ("seasonal_calendar", &["solstice", "equinox"], 0.45, 0.5, &["calendar"]),
    ("star_map", &["star_rising", "lunar_cycle"], 0.55, 0.55, &[]),
    ("eclipse_prediction", &["eclipse_solar", "eclipse_lunar", "lunar_cycle"], 0.65, 0.65, &["mathematics_basic"]),
    ("planetary_model", &["planet_motion", "star_map"], 0.7, 0.7, &["mathematics_basic", "writing_system"]),
];

fn celestial_description(event_id: &str) -> &str {
    match event_id {
        "lunar_cycle" => "The moon completes another cycle of phases",
        "solstice" => "The sun reaches its extreme position",
        "equinox" => "Day and night are of equal length",
        "star_rising" => "A prominent star rises at sunset",
        "eclipse_solar" => "The sun is obscured — a solar eclipse",
        "eclipse_lunar" => "The moon turns blood red — a lunar eclipse",
        "planet_motion" => "A wandering star moves against the fixed stars",
        "comet" => "A bright object with a tail crosses the sky",
        other => other,
    }
}

fn astronomy_knowledge_description(knowledge_id: &str) -> &str {
    match knowledge_id {
        "lunar_tracking" => "The phases of the moon can be predicted",
        "seasonal_calendar" => "A calendar based on sun and moon positions is developed",
        "star_map" => "Named star constellations guide navigation",
        "eclipse_prediction" => "Solar and lunar eclipses can be predicted",
        "planetary_model" => "A model explains the motion of wandering stars",
        other => other,
    }
}

pub fn process_astronomy_tick(
    population: &[crate::state::Individual],
    observations: &mut HashSet<String>,
    astronomy_knowledge: &mut HashSet<String>,
    discovered_techs: &HashSet<String>,
    sim_day: i32,
) -> Vec<Value> {
    let mut events = Vec::new();
    // `phase_offset` keeps events that share the same period from always
    // landing on the same day: solstice/equinox both recur twice a year
    // (~182.5 days apart from their own previous occurrence), but a solstice
    // and the nearest equinox are actually offset by a quarter-year in the
    // real calendar, not simultaneous -- likewise the two eclipse types,
    // which within a shared eclipse season are separated by roughly half a
    // synodic month, not the same day.
    let celestial: [(&str, f64, f64, bool, i32); 8] = [
        ("lunar_cycle", 29.5, 0.9, false, 0),
        ("solstice", 182.5, 0.7, false, 0),
        ("equinox", 182.5, 0.6, false, 91),
        ("star_rising", 365.0, 0.5, false, 0),
        ("eclipse_solar", 177.5, 0.99, false, 0),
        ("eclipse_lunar", 177.5, 0.9, false, 15),
        ("planet_motion", 687.0, 0.4, false, 0),
        ("comet", 3650.0, 0.99, true, 0),
    ];
    for (event_id, period, observability, rare, phase_offset) in celestial {
        if rare {
            // Derived from the event's own declared `period`/`observability`
            // rather than a hardcoded roll -- a daily probability of
            // observability/period gives this event a true expected
            // recurrence of ~period days (weighted by how easy it is to spot
            // when it does happen), matching what the table above actually
            // documents instead of silently diverging from it. Previously
            // this used a flat 0.001 regardless of `period`, giving comet an
            // actual ~1000-day recurrence despite its table row claiming 3650.
            let daily_probability = observability / period;
            if rand::random::<f64>() > daily_probability {
                continue;
            }
            observations.insert(event_id.to_string());
            events.push(json!({ "type": "celestial_observation", "event_id": event_id, "day": sim_day, "importance": "high", "description": celestial_description(event_id) }));
            continue;
        }
        if sim_day > 0 && (sim_day - phase_offset).rem_euclid(period.round() as i32) == 0 && rand::random::<f64>() < observability {
            observations.insert(event_id.to_string());
            events.push(json!({ "type": "celestial_observation", "event_id": event_id, "day": sim_day, "importance": if event_id.contains("eclipse") { "high" } else { "low" }, "description": celestial_description(event_id) }));
        }
    }
    for individual in population.iter().filter(|i| !i.is_dead) {
        // Astronomical knowledge requires enough life experience to reflect on
        // the sky; infants and children never unlock it.
        let life_stage = crate::biology::individual::get_life_stage(individual, sim_day);
        if life_stage == "infant" || life_stage == "child" {
            continue;
        }
        let foxp2 = individual.language.foxp2_expression;
        let iq = individual.phenotype.fluid_intelligence;
        let curiosity = individual.phenotype.curiosity;
        if curiosity <= 0.5 {
            continue;
        }
        for (kid, requires_obs, iq_min, foxp2_min, requires_tech) in ASTRONOMY_KNOWLEDGE {
            if astronomy_knowledge.contains(*kid)
                || iq < *iq_min
                || foxp2 < *foxp2_min
                || requires_obs.iter().any(|obs| !observations.contains(*obs))
                || requires_tech.iter().any(|t| !discovered_techs.contains(*t))
            {
                continue;
            }
            if rand::random::<f64>() < curiosity * iq * 0.0001 {
                astronomy_knowledge.insert((*kid).to_string());
                events.push(json!({ "type": "astronomy_discovery", "knowledge_id": kid, "discoverer_id": individual.id, "day": sim_day, "importance": if *iq_min > 0.6 { "high" } else { "medium" }, "description": astronomy_knowledge_description(kid) }));
            }
        }
    }
    events
}

pub fn get_astronomy_bonus(astronomy_knowledge: &HashSet<String>) -> Value {
    let mut b = serde_json::Map::new();
    let mut add = |key: &str, delta: f64| {
        let current = b.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        b.insert(key.to_string(), json!(current + delta));
    };
    if astronomy_knowledge.contains("lunar_tracking") {
        add("navigation", 0.10);
    }
    if astronomy_knowledge.contains("seasonal_calendar") {
        add("farming_efficiency", 0.15);
    }
    if astronomy_knowledge.contains("star_map") {
        add("navigation", 0.20);
        add("seafaring", 0.20);
    }
    if astronomy_knowledge.contains("eclipse_prediction") {
        add("farming_efficiency", 0.05);
    }
    if astronomy_knowledge.contains("planetary_model") {
        add("navigation", 0.10);
        add("innovation_rate", 0.10);
    }
    Value::Object(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Language, Phenotype};

    fn make_obs(id: &str, curiosity: f64, iq: f64, foxp2: f64, birth_day: i32) -> crate::state::Individual {
        crate::state::Individual {
            id: id.to_string(),
            birth_day,
            phenotype: Phenotype { curiosity, fluid_intelligence: iq, ..Default::default() },
            language: Language { foxp2_expression: foxp2, ..Default::default() },
            ..Default::default()
        }
    }

    fn adult_obs(id: &str, curiosity: f64, iq: f64, foxp2: f64) -> crate::state::Individual {
        make_obs(id, curiosity, iq, foxp2, -25 * 365)
    }

    // ── ASTRONOMY_KNOWLEDGE ─────────────────────────────────────────────

    #[test]
    fn defines_five_knowledge_types() {
        assert_eq!(ASTRONOMY_KNOWLEDGE.len(), 5);
    }

    #[test]
    fn lunar_tracking_has_the_lowest_iq_requirement() {
        let min = ASTRONOMY_KNOWLEDGE.iter().map(|(_, _, iq, ..)| *iq).fold(f64::INFINITY, f64::min);
        let lunar = ASTRONOMY_KNOWLEDGE.iter().find(|(id, ..)| *id == "lunar_tracking").unwrap().2;
        assert_eq!(lunar, min);
    }

    #[test]
    fn planetary_model_requires_math_and_writing() {
        let (_, _, _, _, tech) = ASTRONOMY_KNOWLEDGE.iter().find(|(id, ..)| *id == "planetary_model").unwrap();
        assert!(tech.contains(&"mathematics_basic"));
        assert!(tech.contains(&"writing_system"));
    }

    #[test]
    fn eclipse_prediction_requires_prior_lunar_cycle_observation() {
        let (_, requires_obs, ..) = ASTRONOMY_KNOWLEDGE.iter().find(|(id, ..)| *id == "eclipse_prediction").unwrap();
        assert!(requires_obs.contains(&"lunar_cycle"));
    }

    // ── processAstronomyTick ─────────────────────────────────────────────

    #[test]
    fn does_not_panic_with_an_empty_population() {
        let _events = process_astronomy_tick(&[], &mut HashSet::new(), &mut HashSet::new(), &HashSet::new(), 1);
    }

    #[test]
    fn solstice_and_equinox_do_not_always_land_on_the_same_day() {
        // V-14 regression: both share a 182.5-day period, which without a
        // phase offset made sim_day % 183 == 0 true for both simultaneously
        // on every occurrence -- these must fall on different days,
        // matching how a solstice and the nearest equinox are actually a
        // quarter-year apart in a real calendar.
        let mut solstice_days = Vec::new();
        let mut equinox_days = Vec::new();
        let mut obs = HashSet::new();
        for day in 1..=3000 {
            obs.clear();
            process_astronomy_tick(&[], &mut obs, &mut HashSet::new(), &HashSet::new(), day);
            if obs.contains("solstice") {
                solstice_days.push(day);
            }
            if obs.contains("equinox") {
                equinox_days.push(day);
            }
        }
        assert!(!solstice_days.is_empty() && !equinox_days.is_empty(), "expected both events to fire within 3000 days");
        assert!(solstice_days.iter().all(|d| !equinox_days.contains(d)), "solstice and equinox must never fall on the same day");
    }

    #[test]
    fn the_two_eclipse_types_do_not_always_land_on_the_same_day() {
        let mut solar_days = Vec::new();
        let mut lunar_days = Vec::new();
        let mut obs = HashSet::new();
        for day in 1..=3000 {
            obs.clear();
            process_astronomy_tick(&[], &mut obs, &mut HashSet::new(), &HashSet::new(), day);
            if obs.contains("eclipse_solar") {
                solar_days.push(day);
            }
            if obs.contains("eclipse_lunar") {
                lunar_days.push(day);
            }
        }
        assert!(!solar_days.is_empty() && !lunar_days.is_empty(), "expected both eclipse types to fire within 3000 days");
        assert!(solar_days.iter().all(|d| !lunar_days.contains(d)), "solar and lunar eclipses must never fall on the same day");
    }

    #[test]
    fn comet_recurrence_tracks_its_declared_period_not_a_hardcoded_rate() {
        // The table declares comet's period as 3650 days (~10 years); the
        // daily roll is now derived from period/observability instead of a
        // hardcoded constant, so over a long run the observed recurrence
        // should land in the right order of magnitude (previously it was
        // ~1000 days -- more than 3x too frequent).
        //
        // 300k days (not 50k) so the Bernoulli-trial sample is large enough
        // for occurrence-count variance to stay well clear of the assertion
        // bounds below -- at 50k days the daily draw's own binomial noise put
        // occurrences within its normal range yet outside (2000, 6000) often
        // enough to flake in CI (e.g. 8 occurrences -> 6250).
        let mut obs = HashSet::new();
        let mut occurrences = 0;
        let days = 300_000;
        for day in 1..=days {
            process_astronomy_tick(&[], &mut obs, &mut HashSet::new(), &HashSet::new(), day);
            if obs.remove("comet") {
                occurrences += 1;
            }
        }
        let avg_interval = days as f64 / occurrences.max(1) as f64;
        assert!(occurrences > 0, "expected at least one comet in {days} days");
        assert!((2000.0..6000.0).contains(&avg_interval), "expected average interval near the declared 3650-day period, got {avg_interval} ({occurrences} occurrences in {days} days)");
    }

    #[test]
    fn lunar_cycle_observation_eventually_fires() {
        let mut obs = HashSet::new();
        let mut fired = false;
        for day in 0..=3000 {
            process_astronomy_tick(&[], &mut obs, &mut HashSet::new(), &HashSet::new(), day);
            if obs.contains("lunar_cycle") {
                fired = true;
                break;
            }
        }
        assert!(fired);
    }

    #[test]
    fn infants_and_children_never_unlock_astronomy_knowledge() {
        let mut obs = HashSet::new();
        obs.insert("lunar_cycle".to_string());
        let mut knowledge = HashSet::new();
        for day in 0..5000 {
            // Rebuilt every day so age tracks `day` and they never age past
            // child/infant over the course of the loop.
            let infant = make_obs("i1", 0.9, 0.9, 0.9, day); // always < 1 year old
            let child = make_obs("c1", 0.9, 0.9, 0.9, day - 5 * 365); // always 5 years old
            process_astronomy_tick(&[infant, child], &mut obs, &mut knowledge, &HashSet::new(), day);
        }
        assert!(knowledge.is_empty());
    }

    #[test]
    fn zero_curiosity_observer_never_unlocks_knowledge() {
        let observer = adult_obs("o1", 0.0, 0.8, 0.7);
        let mut obs = HashSet::new();
        obs.insert("lunar_cycle".to_string());
        let mut knowledge = HashSet::new();
        for day in 0..10000 {
            process_astronomy_tick(std::slice::from_ref(&observer), &mut obs, &mut knowledge, &HashSet::new(), day);
        }
        assert!(knowledge.is_empty());
    }

    #[test]
    fn low_iq_blocks_lunar_tracking_specifically() {
        let observer = adult_obs("o1", 0.9, 0.1, 0.9);
        let mut obs = HashSet::new();
        obs.insert("lunar_cycle".to_string());
        let mut knowledge = HashSet::new();
        for day in 0..10000 {
            process_astronomy_tick(std::slice::from_ref(&observer), &mut obs, &mut knowledge, &HashSet::new(), day);
        }
        assert!(!knowledge.contains("lunar_tracking"));
    }

    #[test]
    fn a_capable_observer_can_eventually_discover_lunar_tracking() {
        let observer = adult_obs("o1", 0.99, 0.99, 0.99);
        let mut obs = HashSet::new();
        obs.insert("lunar_cycle".to_string());
        let mut knowledge = HashSet::new();
        let mut found = false;
        for day in 0..200_000 {
            process_astronomy_tick(std::slice::from_ref(&observer), &mut obs, &mut knowledge, &HashSet::new(), day);
            if knowledge.contains("lunar_tracking") {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn astronomy_discovery_event_has_expected_shape() {
        let observer = adult_obs("stargazer", 0.99, 0.99, 0.99);
        let mut obs = HashSet::new();
        obs.insert("lunar_cycle".to_string());
        let mut knowledge = HashSet::new();
        let mut found_event = None;
        for day in 0..200_000 {
            let evs = process_astronomy_tick(std::slice::from_ref(&observer), &mut obs, &mut knowledge, &HashSet::new(), day);
            if let Some(ev) = evs.into_iter().find(|e| e["type"] == "astronomy_discovery") {
                found_event = Some(ev);
                break;
            }
        }
        if let Some(ev) = found_event {
            assert_eq!(ev["discoverer_id"], "stargazer");
            assert!(ev["knowledge_id"].is_string());
        }
    }

    #[test]
    fn seasonal_calendar_requires_calendar_tech() {
        let observer = adult_obs("o1", 0.99, 0.99, 0.99);
        let mut obs = HashSet::new();
        obs.insert("solstice".to_string());
        obs.insert("equinox".to_string());
        let mut knowledge = HashSet::new();
        for day in 0..50_000 {
            process_astronomy_tick(std::slice::from_ref(&observer), &mut obs, &mut knowledge, &HashSet::new(), day);
        }
        assert!(!knowledge.contains("seasonal_calendar"));
    }

    // ── getAstronomyBonus ────────────────────────────────────────────────

    #[test]
    fn empty_knowledge_yields_no_bonuses() {
        let bonus = get_astronomy_bonus(&HashSet::new());
        assert!(bonus.as_object().unwrap().is_empty());
    }

    #[test]
    fn lunar_tracking_grants_navigation_bonus() {
        let mut k = HashSet::new();
        k.insert("lunar_tracking".to_string());
        assert!(get_astronomy_bonus(&k)["navigation"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn seasonal_calendar_grants_farming_bonus() {
        let mut k = HashSet::new();
        k.insert("seasonal_calendar".to_string());
        assert!(get_astronomy_bonus(&k)["farming_efficiency"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn star_map_grants_navigation_and_seafaring() {
        let mut k = HashSet::new();
        k.insert("star_map".to_string());
        let bonus = get_astronomy_bonus(&k);
        assert!(bonus["navigation"].as_f64().unwrap() > 0.0);
        assert!(bonus["seafaring"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn full_knowledge_set_accumulates_all_bonuses() {
        let all: HashSet<String> = ["lunar_tracking", "seasonal_calendar", "star_map", "eclipse_prediction", "planetary_model"].iter().map(|s| s.to_string()).collect();
        let bonus = get_astronomy_bonus(&all);
        assert!(bonus["navigation"].as_f64().unwrap() > 0.3);
        assert!(bonus["farming_efficiency"].as_f64().unwrap() > 0.15);
        assert!(bonus["innovation_rate"].as_f64().unwrap() > 0.0);
    }
}
