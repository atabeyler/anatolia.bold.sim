//! God-mode intervention logic, factored out of sim-server's HTTP layer so it
//! has exactly one implementation shared by the native server and any other
//! caller (e.g. a WASM build with no server at all) instead of two copies
//! that could quietly drift apart. Pure and DB/HTTP-free by design so the
//! Cardinal Rule constraints (genetic_boost/longevity only ever touch
//! is_founder=true individuals; death is never blocked) can be unit-tested
//! directly, without mocking an HTTP route.
use serde_json::{json, Value};

pub fn mark_dead(individual: &mut Value, day: i64, cause: &str) {
    if let Some(obj) = individual.as_object_mut() {
        obj.insert("is_dead".to_string(), json!(true));
        obj.insert("alive".to_string(), json!(false));
        obj.insert("death_day".to_string(), json!(day));
        obj.insert("death_cause".to_string(), json!(cause));
        let health = obj.entry("health").or_insert_with(|| json!({}));
        if let Some(h) = health.as_object_mut() {
            h.insert("hp".to_string(), json!(0));
        }
    }
}

/// Equirectangular approximation, not full haversine trig -- accurate enough
/// at the regional scale (radius up to a few hundred km) these disasters
/// operate at, and much cheaper for what's already a rare, low-frequency
/// admin action rather than a per-tick hot path.
pub fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let avg_lat_rad = ((lat1 + lat2) / 2.0).to_radians();
    let dx = (lon2 - lon1) * 111.32 * avg_lat_rad.cos();
    let dy = (lat2 - lat1) * 110.57;
    (dx * dx + dy * dy).sqrt()
}

/// Shared mortality application for the four geographically-targeted
/// disasters (earthquake/flood/volcano/meteor): individuals within
/// `radius_km` of (lat, lon) -- reading each individual's own `x`/`y`
/// (lon/lat degrees, see AGENTS.md) -- face a mortality roll that peaks at
/// `base_mortality` at the epicenter and falls off linearly to zero at the
/// radius edge. Founders get the same 0.5x discount every other death path
/// in the simulation gives them (environment::process_disaster,
/// mortality.rs) -- these are meant to be dramatic rare events, not a
/// reliable way to snipe a founder.
pub fn apply_geo_disaster(sim: &mut Value, lat: f64, lon: f64, radius_km: f64, base_mortality: f64, day: i64, cause: &str) -> (i64, i64) {
    let mut affected = 0_i64;
    let mut deaths = 0_i64;
    let radius_km = radius_km.max(1.0);
    let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) else {
        return (0, 0);
    };
    for ind in inds.iter_mut() {
        if ind.get("is_dead").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let x = ind.get("x").and_then(Value::as_f64).unwrap_or(lon);
        let y = ind.get("y").and_then(Value::as_f64).unwrap_or(lat);
        let dist = distance_km(lat, lon, y, x);
        if dist > radius_km {
            continue;
        }
        affected += 1;
        let falloff = 1.0 - (dist / radius_km);
        let is_founder = ind.get("is_founder").and_then(Value::as_bool).unwrap_or(false);
        let founder_factor = if is_founder { 0.5 } else { 1.0 };
        let risk = (base_mortality * falloff * founder_factor).clamp(0.0, 1.0);
        if rand::random::<f64>() < risk {
            mark_dead(ind, day, cause);
            deaths += 1;
        }
    }
    (affected, deaths)
}

/// Applies one god-mode intervention's business logic to an in-memory simulation
/// snapshot. Pure and DB/HTTP-free by design so the Cardinal Rule constraints
/// (genetic_boost/longevity only ever touch is_founder=true individuals; death is
/// never blocked) can be unit-tested directly, mirroring how the original JS test
/// suite validated this logic without mocking the HTTP route.
pub fn apply_intervention(sim: &mut Value, intervention_type: &str, params: &Value, day: i64, alive_count: usize) -> Result<(i64, i64), String> {
    let mut affected = 0_i64;
    let mut deaths = 0_i64;

    match intervention_type {
        "instant_death" => {
            if let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) {
                if let Some(id) = params.get("individual_id").and_then(|v| v.as_str()) {
                    if let Some(ind) = inds.iter_mut().find(|i| i.get("id").and_then(|v| v.as_str()) == Some(id)) {
                        if !ind.get("is_dead").and_then(|v| v.as_bool()).unwrap_or(false) {
                            mark_dead(ind, day, "god_intervention");
                            affected = 1;
                            deaths = 1;
                        }
                    }
                }
            }
        }
        "genetic_boost" => {
            if let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) {
                if let Some(id) = params.get("individual_id").and_then(|v| v.as_str()) {
                    if let Some(ind) = inds.iter_mut().find(|i| i.get("id").and_then(|v| v.as_str()) == Some(id)) {
                        if !ind.get("is_founder").and_then(|v| v.as_bool()).unwrap_or(false) {
                            return Err("Genetic boost only applies to founders — simulation integrity rule".to_string());
                        }
                        let trait_name = params.get("trait").and_then(|v| v.as_str()).unwrap_or("fluid_intelligence");
                        let amount = params.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.1);
                        let phenotype = ind.as_object_mut().unwrap();
                        let ph = phenotype.entry("phenotype").or_insert_with(|| json!({}));
                        if let Some(obj) = ph.as_object_mut() {
                            let current = obj.get(trait_name).and_then(|v| v.as_f64()).unwrap_or(0.5);
                            obj.insert(trait_name.to_string(), json!((current + amount).min(1.0)));
                            affected = 1;
                        }
                    }
                }
            }
        }
        "longevity" => {
            if let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) {
                if let Some(id) = params.get("individual_id").and_then(|v| v.as_str()) {
                    if let Some(ind) = inds.iter_mut().find(|i| i.get("id").and_then(|v| v.as_str()) == Some(id)) {
                        if !ind.get("is_founder").and_then(|v| v.as_bool()).unwrap_or(false) {
                            return Err("Longevity boost only applies to founders — Cardinal Rule: non-founder phenotypes may not be directly modified".to_string());
                        }
                        let extra_years = params.get("extra_years").and_then(|v| v.as_f64()).unwrap_or(50.0);
                        let phenotype = ind.as_object_mut().unwrap();
                        let ph = phenotype.entry("phenotype").or_insert_with(|| json!({}));
                        if let Some(obj) = ph.as_object_mut() {
                            let current = obj.get("max_lifespan").and_then(|v| v.as_f64()).unwrap_or(70.0);
                            obj.insert("max_lifespan".to_string(), json!((current + extra_years).min(200.0)));
                            affected = 1;
                        }
                    }
                }
            }
        }
        "resource_boost" => {
            if let Some(world) = sim.get_mut("world_state").and_then(|v| v.as_object_mut()) {
                let food = params.get("food").and_then(|v| v.as_f64()).unwrap_or(0.2);
                let water = params.get("water").and_then(|v| v.as_f64()).unwrap_or(0.2);
                world.insert("food_abundance".to_string(), json!((world.get("food_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5) + food * 0.1).min(1.0)));
                world.insert("water_abundance".to_string(), json!((world.get("water_abundance").and_then(|v| v.as_f64()).unwrap_or(0.7) + water * 0.1).min(1.0)));
                affected = alive_count as i64;
            }
        }
        "drought" => {
            if let Some(world) = sim.get_mut("world_state").and_then(|v| v.as_object_mut()) {
                world.insert("food_abundance".to_string(), json!(0.1));
                world.insert("water_abundance".to_string(), json!(0.05));
                affected = alive_count as i64;
            }
        }
        // Each is geographically targeted (except epidemic, which has no
        // lat/lon in its own params) using each individual's own x/y position
        // rather than the whole population uniformly, matching how a real
        // localized disaster would only strike whoever is actually near it.
        "earthquake" => {
            let lat = params.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = params.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let radius = params.get("radius").and_then(|v| v.as_f64()).unwrap_or(100.0);
            let magnitude = params.get("magnitude").and_then(|v| v.as_f64()).unwrap_or(6.0);
            // Richter-like scale: negligible below ~magnitude 4, catastrophic
            // (~40% peak mortality at the epicenter) by magnitude 9.
            let base_mortality = ((magnitude - 4.0) / 5.0).clamp(0.0, 1.0) * 0.4;
            let (a, d) = apply_geo_disaster(sim, lat, lon, radius, base_mortality, day, "earthquake");
            affected = a;
            deaths = d;
        }
        "flood" => {
            let lat = params.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = params.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let radius = params.get("radius").and_then(|v| v.as_f64()).unwrap_or(150.0);
            let severity = params.get("severity").and_then(|v| v.as_f64()).unwrap_or(0.5).clamp(0.0, 1.0);
            let base_mortality = severity * 0.3;
            let (a, d) = apply_geo_disaster(sim, lat, lon, radius, base_mortality, day, "flood");
            affected = a;
            deaths = d;
            // Matches environment::process_disaster's own flood handling:
            // survivors within the flood radius pick up standing-water state
            // (raised water fear, temporary _inWater), not just a one-off
            // death roll -- so drowning risk/water-fear behavior for the
            // following few days is consistent regardless of whether the
            // flood was a natural or a god-mode event.
            if let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) {
                for ind in inds.iter_mut() {
                    if ind.get("is_dead").and_then(Value::as_bool).unwrap_or(false) {
                        continue;
                    }
                    let x = ind.get("x").and_then(Value::as_f64).unwrap_or(lon);
                    let y = ind.get("y").and_then(Value::as_f64).unwrap_or(lat);
                    if distance_km(lat, lon, y, x) > radius.max(1.0) {
                        continue;
                    }
                    if let Some(obj) = ind.as_object_mut() {
                        let extra = obj.entry("extra").or_insert_with(|| json!({}));
                        if let Some(extra) = extra.as_object_mut() {
                            let water_fear = extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
                            extra.insert("_waterFear".to_string(), json!((water_fear + 0.3).min(1.0)));
                            extra.insert("_inWater".to_string(), json!(true));
                            extra.insert("_wasInWater".to_string(), json!(true));
                            extra.insert("_inWaterDaysRemaining".to_string(), json!(3));
                        }
                    }
                }
            }
        }
        "volcano" => {
            let lat = params.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = params.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let radius = params.get("radius").and_then(|v| v.as_f64()).unwrap_or(200.0);
            let power = params.get("power").and_then(|v| v.as_f64()).unwrap_or(7.0);
            // VEI-like scale: an eruption is more locally devastating than an
            // equivalent-magnitude earthquake (lava/pyroclastic flow near the
            // epicenter), capping higher -- ~50% peak mortality by power 10.
            let base_mortality = ((power - 3.0) / 7.0).clamp(0.0, 1.0) * 0.5;
            let (a, d) = apply_geo_disaster(sim, lat, lon, radius, base_mortality, day, "volcano");
            affected = a;
            deaths = d;
        }
        "meteor" => {
            let lat = params.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = params.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            // No radius param from the client for this one -- a meteor's
            // devastation radius scales with its own size instead of being
            // independently chosen.
            let size = params.get("size").and_then(|v| v.as_f64()).unwrap_or(3.0).max(0.1);
            let radius = (size * 20.0).clamp(10.0, 300.0);
            let base_mortality = (size / 10.0).clamp(0.0, 1.0) * 0.6;
            let (a, d) = apply_geo_disaster(sim, lat, lon, radius, base_mortality, day, "meteor");
            affected = a;
            deaths = d;
        }
        "epidemic" => {
            // No lat/lon in this one's own params -- an epidemic spreads
            // through the whole population, not a fixed radius. spread_rate
            // is each individual's own exposure chance; mortality_rate is the
            // risk for whoever actually gets exposed, mirroring how a real
            // outbreak's case count and case-fatality rate are two
            // independent numbers.
            let mortality_rate = params.get("mortality_rate").and_then(|v| v.as_f64()).unwrap_or(0.2).clamp(0.0, 1.0);
            let spread_rate = params.get("spread_rate").and_then(|v| v.as_f64()).unwrap_or(0.5).clamp(0.0, 1.0);
            if let Some(inds) = sim.get_mut("individuals").and_then(|v| v.as_array_mut()) {
                for ind in inds.iter_mut() {
                    if ind.get("is_dead").and_then(Value::as_bool).unwrap_or(false) {
                        continue;
                    }
                    if rand::random::<f64>() >= spread_rate {
                        continue;
                    }
                    affected += 1;
                    let is_founder = ind.get("is_founder").and_then(Value::as_bool).unwrap_or(false);
                    let founder_factor = if is_founder { 0.5 } else { 1.0 };
                    if rand::random::<f64>() < (mortality_rate * founder_factor).clamp(0.0, 1.0) {
                        mark_dead(ind, day, "epidemic");
                        deaths += 1;
                    }
                }
            }
        }
        "weather_override" => {
            if let Some(world) = sim.get_mut("world_state").and_then(|v| v.as_object_mut()) {
                world.insert("current_weather".to_string(), json!(params.get("weather").and_then(|v| v.as_str()).unwrap_or("clear")));
                world.insert("weather_intensity".to_string(), json!(params.get("intensity").and_then(|v| v.as_f64()).unwrap_or(0.5)));
                affected = alive_count as i64;
            }
        }
        "quarantine" => {
            // Must land in world_state (flattened into the top-level world
            // JSON environment::natural_disaster_probability reads via
            // "quarantine_mode"), not the simulation's own top-level `extra`
            // -- writing there left this toggle checked nowhere, silently
            // doing nothing.
            let enabled = params.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            if let Some(world) = sim.get_mut("world_state").and_then(|v| v.as_object_mut()) {
                world.insert("quarantine_mode".to_string(), json!(enabled));
                affected = alive_count as i64;
            }
        }
        "talk" => {
            affected = 1;
        }
        other => {
            return Err(format!("Unknown intervention type: {other}"));
        }
    }

    if let Some(sim_obj) = sim.as_object_mut() {
        let extra = sim_obj.entry("extra").or_insert_with(|| json!({}));
        if let Some(extra) = extra.as_object_mut() {
            extra.insert("intervened".to_string(), json!(true));
        }
    }

    Ok((affected, deaths))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_founder() -> Value {
        json!({
            "id": "founder-1",
            "is_founder": true,
            "is_dead": false,
            "alive": true,
            "phenotype": { "fluid_intelligence": 0.7, "language_capacity": 0.8, "immune_strength": 0.6, "max_lifespan": 70.0, "aggression": 0.4 },
            "health": { "hp": 1.0, "calories": 1.0, "hydration": 1.0 },
        })
    }

    fn make_non_founder() -> Value {
        json!({
            "id": "child-1",
            "is_founder": false,
            "is_dead": false,
            "alive": true,
            "birth_day": 100,
            "phenotype": { "fluid_intelligence": 0.6, "language_capacity": 0.7, "immune_strength": 0.5, "max_lifespan": 65.0, "aggression": 0.3 },
            "health": { "hp": 1.0, "calories": 1.0, "hydration": 1.0 },
        })
    }

    fn sim_with(individuals: Vec<Value>) -> Value {
        json!({ "individuals": individuals, "current_day": 100, "world_state": {} })
    }

    // ── instant_death ────────────────────────────────────────────────────

    #[test]
    fn instant_death_works_on_founder() {
        let mut sim = sim_with(vec![make_founder()]);
        let (affected, deaths) = apply_intervention(&mut sim, "instant_death", &json!({"individual_id": "founder-1"}), 100, 1).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(deaths, 1);
        assert_eq!(sim["individuals"][0]["is_dead"], true);
        assert_eq!(sim["individuals"][0]["death_cause"], "god_intervention");
    }

    #[test]
    fn instant_death_works_on_non_founder_cardinal_rule_does_not_block_death() {
        // Direct death is in the same category as natural disasters -- the
        // Cardinal Rule forbids behavior injection, not death events.
        let mut sim = sim_with(vec![make_non_founder()]);
        let (affected, deaths) = apply_intervention(&mut sim, "instant_death", &json!({"individual_id": "child-1"}), 200, 1).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(deaths, 1);
        assert_eq!(sim["individuals"][0]["is_dead"], true);
        assert_eq!(sim["individuals"][0]["alive"], false);
    }

    // ── quarantine ──────────────────────────────────────────────────────

    #[test]
    fn quarantine_writes_to_world_state_not_the_simulations_own_extra() {
        let mut sim = sim_with(vec![make_founder()]);
        let (affected, _) = apply_intervention(&mut sim, "quarantine", &json!({"enabled": true}), 100, 1).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(sim["world_state"]["quarantine_mode"], true, "quarantine must land in world_state, not sim.extra, or nothing ever reads it");
    }

    #[test]
    fn quarantine_actually_suppresses_natural_disaster_probability() {
        // End-to-end regression: not just that the flag lands in the right
        // place, but that the exact function tick.rs calls to roll disasters
        // (environment::natural_disaster_probability) reads it back as
        // intended and drops to zero.
        let mut sim = sim_with(vec![make_founder()]);
        apply_intervention(&mut sim, "quarantine", &json!({"enabled": true}), 100, 1).unwrap();
        let world_state = sim.get("world_state").cloned().unwrap_or_default();
        assert_eq!(crate::environment::natural_disaster_probability(&world_state), 0.0);
    }

    #[test]
    fn instant_death_labels_cause_distinguishably_from_natural_death() {
        let mut sim = sim_with(vec![make_non_founder()]);
        apply_intervention(&mut sim, "instant_death", &json!({"individual_id": "child-1"}), 200, 1).unwrap();
        let cause = sim["individuals"][0]["death_cause"].as_str().unwrap();
        assert_eq!(cause, "god_intervention");
        assert_ne!(cause, "old_age");
        assert_ne!(cause, "starvation");
    }

    #[test]
    fn instant_death_is_a_no_op_for_an_already_dead_individual() {
        let mut dead = make_non_founder();
        dead["is_dead"] = json!(true);
        dead["alive"] = json!(false);
        dead["death_day"] = json!(50);
        let mut sim = sim_with(vec![dead]);
        let (affected, deaths) = apply_intervention(&mut sim, "instant_death", &json!({"individual_id": "child-1"}), 200, 1).unwrap();
        assert_eq!(affected, 0);
        assert_eq!(deaths, 0);
        assert_eq!(sim["individuals"][0]["death_day"], 50);
    }

    // ── genetic_boost — Cardinal Rule ───────────────────────────────────

    #[test]
    fn genetic_boost_works_on_founder() {
        let mut sim = sim_with(vec![make_founder()]);
        let (affected, _) = apply_intervention(&mut sim, "genetic_boost", &json!({"individual_id": "founder-1", "trait": "fluid_intelligence", "amount": 0.1}), 100, 1).unwrap();
        assert_eq!(affected, 1);
        assert!(sim["individuals"][0]["phenotype"]["fluid_intelligence"].as_f64().unwrap() > 0.7);
    }

    #[test]
    fn genetic_boost_is_rejected_for_non_founder() {
        let mut sim = sim_with(vec![make_non_founder()]);
        let before = sim["individuals"][0]["phenotype"]["fluid_intelligence"].clone();
        let err = apply_intervention(&mut sim, "genetic_boost", &json!({"individual_id": "child-1", "trait": "fluid_intelligence", "amount": 0.2}), 100, 1).unwrap_err();
        assert!(err.contains("founders"));
        // Phenotype must remain untouched -- the whole snapshot is discarded on error.
        assert_eq!(sim["individuals"][0]["phenotype"]["fluid_intelligence"], before);
    }

    #[test]
    fn genetic_boost_is_capped_at_one() {
        let mut founder = make_founder();
        founder["phenotype"]["fluid_intelligence"] = json!(0.95);
        let mut sim = sim_with(vec![founder]);
        apply_intervention(&mut sim, "genetic_boost", &json!({"individual_id": "founder-1", "trait": "fluid_intelligence", "amount": 0.2}), 100, 1).unwrap();
        assert_eq!(sim["individuals"][0]["phenotype"]["fluid_intelligence"], 1.0);
    }

    // ── longevity — Cardinal Rule ────────────────────────────────────────

    #[test]
    fn longevity_is_rejected_for_non_founder() {
        let mut sim = sim_with(vec![make_non_founder()]);
        let err = apply_intervention(&mut sim, "longevity", &json!({"individual_id": "child-1"}), 100, 1).unwrap_err();
        assert!(err.contains("Cardinal Rule"));
    }

    #[test]
    fn longevity_works_for_founder_and_is_capped_at_two_hundred() {
        let mut founder = make_founder();
        founder["phenotype"]["max_lifespan"] = json!(180.0);
        let mut sim = sim_with(vec![founder]);
        apply_intervention(&mut sim, "longevity", &json!({"individual_id": "founder-1", "extra_years": 50}), 100, 1).unwrap();
        assert_eq!(sim["individuals"][0]["phenotype"]["max_lifespan"], 200.0);
    }

    // ── Cardinal Rule summary ────────────────────────────────────────────

    #[test]
    fn unknown_intervention_type_is_rejected() {
        let mut sim = sim_with(vec![make_founder()]);
        assert!(apply_intervention(&mut sim, "not_a_real_intervention", &json!({}), 100, 1).is_err());
    }

    // ── geographically-targeted disasters ────────────────────────────────
    // earthquake/flood/volcano/meteor/epidemic used to fall through to
    // "Unknown intervention type" (a 400) -- the client has sent these five
    // types since before any of them existed server-side.

    fn make_individual_at(id: &str, is_founder: bool, x: f64, y: f64) -> Value {
        json!({
            "id": id,
            "is_founder": is_founder,
            "is_dead": false,
            "alive": true,
            "x": x,
            "y": y,
            "phenotype": { "fluid_intelligence": 0.6, "language_capacity": 0.7, "immune_strength": 0.5, "max_lifespan": 65.0, "aggression": 0.3, "endurance": 0.5, "physical_strength": 0.5 },
            "health": { "hp": 1.0, "calories": 1.0, "hydration": 1.0 },
        })
    }

    #[test]
    fn earthquake_at_maximum_magnitude_can_kill_someone_at_the_epicenter() {
        let mut deaths_seen = false;
        for _ in 0..200 {
            let mut sim = sim_with(vec![make_individual_at("i1", false, 35.0, 38.0)]);
            let (affected, deaths) = apply_intervention(&mut sim, "earthquake", &json!({"lat": 38.0, "lon": 35.0, "radius": 100, "magnitude": 9}), 100, 1).unwrap();
            assert_eq!(affected, 1, "someone at the exact epicenter must always be counted as affected");
            if deaths > 0 {
                deaths_seen = true;
                assert_eq!(sim["individuals"][0]["death_cause"], "earthquake");
                break;
            }
        }
        assert!(deaths_seen, "a magnitude-9 earthquake at the epicenter should kill someone within 200 trials");
    }

    #[test]
    fn earthquake_never_affects_someone_far_outside_its_radius() {
        // ~1 degree of latitude is roughly 111km, so 10 degrees away is
        // nowhere near a 100km-radius quake.
        let mut sim = sim_with(vec![make_individual_at("far", false, 35.0, 48.0)]);
        let (affected, deaths) = apply_intervention(&mut sim, "earthquake", &json!({"lat": 38.0, "lon": 35.0, "radius": 100, "magnitude": 9}), 100, 1).unwrap();
        assert_eq!(affected, 0);
        assert_eq!(deaths, 0);
        assert!(!sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false));
    }

    #[test]
    fn zero_magnitude_earthquake_affects_but_never_kills() {
        let mut sim = sim_with(vec![make_individual_at("i1", false, 35.0, 38.0)]);
        let (affected, deaths) = apply_intervention(&mut sim, "earthquake", &json!({"lat": 38.0, "lon": 35.0, "radius": 100, "magnitude": 2}), 100, 1).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(deaths, 0);
    }

    #[test]
    fn flood_raises_water_fear_and_marks_survivors_in_water_within_its_radius() {
        let mut sim = sim_with(vec![make_individual_at("i1", false, 35.0, 38.0)]);
        apply_intervention(&mut sim, "flood", &json!({"lat": 38.0, "lon": 35.0, "radius": 150, "severity": 0.0}), 100, 1).unwrap();
        assert!(!sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false), "zero severity must never kill");
        assert_eq!(sim["individuals"][0]["extra"]["_waterFear"], 0.3);
        assert_eq!(sim["individuals"][0]["extra"]["_inWater"], true);
    }

    #[test]
    fn flood_does_not_touch_water_fear_for_someone_outside_its_radius() {
        let mut sim = sim_with(vec![make_individual_at("far", false, 35.0, 48.0)]);
        apply_intervention(&mut sim, "flood", &json!({"lat": 38.0, "lon": 35.0, "radius": 150, "severity": 0.5}), 100, 1).unwrap();
        assert!(sim["individuals"][0].get("extra").is_none() || sim["individuals"][0]["extra"].get("_waterFear").is_none());
    }

    #[test]
    fn volcano_and_meteor_are_geographically_gated_the_same_way() {
        for disaster in ["volcano", "meteor"] {
            let mut near = sim_with(vec![make_individual_at("near", false, 35.0, 38.0)]);
            let params = if disaster == "volcano" { json!({"lat": 38.0, "lon": 35.0, "radius": 200, "power": 10}) } else { json!({"lat": 38.0, "lon": 35.0, "size": 10}) };
            let (affected_near, _) = apply_intervention(&mut near, disaster, &params, 100, 1).unwrap();
            assert_eq!(affected_near, 1, "{disaster} should affect someone at the epicenter");

            let mut far = sim_with(vec![make_individual_at("far", false, 35.0, 58.0)]);
            let (affected_far, deaths_far) = apply_intervention(&mut far, disaster, &params, 100, 1).unwrap();
            assert_eq!(affected_far, 0, "{disaster} should not affect someone far outside its radius");
            assert_eq!(deaths_far, 0);
        }
    }

    #[test]
    fn earthquake_founders_die_less_often_than_non_founders_at_the_same_epicenter() {
        const TRIALS: usize = 500;
        let mut founder_deaths = 0;
        let mut non_founder_deaths = 0;
        for _ in 0..TRIALS {
            let mut founder_sim = sim_with(vec![make_individual_at("f1", true, 35.0, 38.0)]);
            let mut non_founder_sim = sim_with(vec![make_individual_at("n1", false, 35.0, 38.0)]);
            let params = json!({"lat": 38.0, "lon": 35.0, "radius": 100, "magnitude": 8});
            apply_intervention(&mut founder_sim, "earthquake", &params, 100, 1).unwrap();
            apply_intervention(&mut non_founder_sim, "earthquake", &params, 100, 1).unwrap();
            if founder_sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false) {
                founder_deaths += 1;
            }
            if non_founder_sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false) {
                non_founder_deaths += 1;
            }
        }
        assert!(
            founder_deaths < non_founder_deaths,
            "founders ({founder_deaths}/{TRIALS}) should die less often than non-founders ({non_founder_deaths}/{TRIALS}) at the same epicenter"
        );
    }

    #[test]
    fn epidemic_with_full_spread_and_mortality_kills_everyone_non_founder() {
        let individuals = vec![make_individual_at("n1", false, 0.0, 0.0), make_individual_at("n2", false, 1.0, 1.0)];
        let mut sim = sim_with(individuals);
        let (affected, deaths) = apply_intervention(&mut sim, "epidemic", &json!({"mortality_rate": 1.0, "spread_rate": 1.0}), 100, 2).unwrap();
        assert_eq!(affected, 2);
        assert_eq!(deaths, 2);
        assert_eq!(sim["individuals"][0]["death_cause"], "epidemic");
    }

    #[test]
    fn epidemic_with_zero_spread_rate_affects_nobody() {
        let mut sim = sim_with(vec![make_individual_at("n1", false, 0.0, 0.0)]);
        let (affected, deaths) = apply_intervention(&mut sim, "epidemic", &json!({"mortality_rate": 1.0, "spread_rate": 0.0}), 100, 1).unwrap();
        assert_eq!(affected, 0);
        assert_eq!(deaths, 0);
    }

    #[test]
    fn epidemic_gives_founders_the_same_mortality_discount() {
        const TRIALS: usize = 500;
        let mut founder_deaths = 0;
        let mut non_founder_deaths = 0;
        for _ in 0..TRIALS {
            let mut founder_sim = sim_with(vec![make_individual_at("f1", true, 0.0, 0.0)]);
            let mut non_founder_sim = sim_with(vec![make_individual_at("n1", false, 0.0, 0.0)]);
            let params = json!({"mortality_rate": 0.9, "spread_rate": 1.0});
            apply_intervention(&mut founder_sim, "epidemic", &params, 100, 1).unwrap();
            apply_intervention(&mut non_founder_sim, "epidemic", &params, 100, 1).unwrap();
            if founder_sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false) {
                founder_deaths += 1;
            }
            if non_founder_sim["individuals"][0]["is_dead"].as_bool().unwrap_or(false) {
                non_founder_deaths += 1;
            }
        }
        assert!(founder_deaths < non_founder_deaths, "founders ({founder_deaths}/{TRIALS}) should die less often than non-founders ({non_founder_deaths}/{TRIALS}) in the same epidemic");
    }

    #[test]
    fn already_dead_individuals_are_never_recounted_by_any_new_disaster_type() {
        for disaster in ["earthquake", "flood", "volcano", "meteor", "epidemic"] {
            let mut dead = make_individual_at("d1", false, 38.0, 35.0);
            dead["is_dead"] = json!(true);
            dead["alive"] = json!(false);
            let mut sim = sim_with(vec![dead]);
            let params = json!({"lat": 38.0, "lon": 35.0, "radius": 500, "magnitude": 9, "power": 10, "size": 10, "severity": 1.0, "mortality_rate": 1.0, "spread_rate": 1.0});
            let (affected, deaths) = apply_intervention(&mut sim, disaster, &params, 100, 1).unwrap();
            assert_eq!(affected, 0, "{disaster} must not re-affect an already-dead individual");
            assert_eq!(deaths, 0);
        }
    }
}
