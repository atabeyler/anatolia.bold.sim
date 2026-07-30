use serde_json::{json, Value};

const BIOMES: &[(&str, [f64; 2], f64, f64, f64)] = &[
    ("tropical_rainforest", [22.0, 30.0], 0.90, 0.95, 0.40),
    ("tropical_savanna", [20.0, 32.0], 0.70, 0.50, 0.50),
    ("desert", [5.0, 45.0], 0.20, 0.10, 0.20),
    ("mediterranean", [8.0, 30.0], 0.75, 0.65, 0.20),
    ("temperate_forest", [-5.0, 25.0], 0.70, 0.75, 0.25),
    ("grassland", [-10.0, 30.0], 0.60, 0.40, 0.35),
    ("boreal_forest", [-30.0, 20.0], 0.50, 0.70, 0.30),
    ("tundra", [-40.0, 10.0], 0.20, 0.60, 0.20),
    ("mountain", [-20.0, 15.0], 0.40, 0.80, 0.30),
    ("coastal", [5.0, 25.0], 0.85, 0.90, 0.15),
];

/// A real land/ocean raster (Natural Earth 10m land polygons minus 10m lakes,
/// rasterized at 0.1-degree resolution, 1 bit/cell, row-major from the south
/// pole/antimeridian) so migrating individuals don't wander into open water --
/// including inland seas and major lakes (Mediterranean, Black Sea, Caspian,
/// the Great Lakes, ...) that a coarse continent-bounding-box approximation
/// would miss. See rust/sim-core/assets/land_mask_0p1deg.bin.
const LAND_MASK: &[u8] = include_bytes!("../assets/land_mask_0p1deg.bin");
const MASK_STEP: f64 = 0.1;
const MASK_LAT_CELLS: usize = 1800;
const MASK_LON_CELLS: usize = 3600;

pub fn is_on_land(latitude: f64, longitude: f64) -> bool {
    let lat = latitude.clamp(-90.0, 89.999);
    let lon = ((longitude + 180.0).rem_euclid(360.0)) - 180.0;
    let row = (((lat + 90.0) / MASK_STEP) as usize).min(MASK_LAT_CELLS - 1);
    let col = (((lon + 180.0) / MASK_STEP) as usize).min(MASK_LON_CELLS - 1);
    let bit_index = row * MASK_LON_CELLS + col;
    let byte = LAND_MASK[bit_index / 8];
    (byte >> (7 - (bit_index % 8))) & 1 == 1
}

pub fn get_biome(latitude: f64, longitude: f64) -> &'static str {
    let abs_lat = latitude.abs();
    let lon_mod = longitude.rem_euclid(90.0).abs();
    let coastal = !(20.0..=70.0).contains(&lon_mod);
    let continental = (35.0..=55.0).contains(&lon_mod);
    if abs_lat < 10.0 { return if coastal { "coastal" } else { "tropical_rainforest" }; }
    if abs_lat < 20.0 { return if continental { "tropical_savanna" } else { "tropical_rainforest" }; }
    if abs_lat < 30.0 { return if coastal { "coastal" } else if continental { "desert" } else { "tropical_savanna" }; }
    if abs_lat < 45.0 { return if coastal { "mediterranean" } else if continental { "grassland" } else { "temperate_forest" }; }
    if abs_lat < 60.0 { return if coastal { "temperate_forest" } else if continental { "grassland" } else { "boreal_forest" }; }
    if abs_lat < 70.0 { return "boreal_forest"; }
    "tundra"
}

pub fn create_world_state(latitude: f64, longitude: f64) -> Value {
    let biome_key = get_biome(latitude, longitude);
    let biome = BIOMES.iter().find(|(k, ..)| *k == biome_key).unwrap_or(&BIOMES[0]);
    let phonology_seed = (((latitude * 100.0).round() as i64) * 31 + ((longitude * 100.0).round() as i64) * 17 + 1277).rem_euclid(10_000);
    json!({
        "latitude": latitude,
        "longitude": longitude,
        "biome": biome_key,
        "temperature": (biome.1[0] + biome.1[1]) / 2.0,
        "food_abundance": biome.2,
        "water_abundance": biome.3,
        "predator_risk": biome.4,
        "disease_pressure": 0.1,
        "season": "spring",
        "day_of_year": 0,
        "year": 0,
        "natural_disaster": null,
        "flora": { "density": biome.2 * 0.8 },
        "fauna": { "prey_density": biome.2 * 0.6, "predator_density": biome.4 },
        "human_impact": 0,
        "phonology_seed": phonology_seed,
        "current_weather": "clear",
        "weather_intensity": 0.5,
        "weather_days_remaining": 5,
        "weather_move_mult": 1.0,
        "weather_hp_delta": 0.0,
        "weather_cold_risk": false,
        "weather_heat_risk": false,
        "_weather_water_delta": 0.0,
        "_weather_food_delta": 0.0
    })
}

pub fn update_world_state(world_state: &mut Value, simulation_day: i32, discovered_techs: Option<&std::collections::HashSet<String>>, population_size: usize) {
    let day_of_year = simulation_day.rem_euclid(365);
    if let Some(obj) = world_state.as_object_mut() {
        obj.insert("day_of_year".to_string(), json!(day_of_year));
        obj.insert("year".to_string(), json!(simulation_day / 365));
        let season = if !(80..335).contains(&day_of_year) { "winter" }
            else if day_of_year < 172 { "spring" }
            else if day_of_year < 264 { "summer" }
            else { "autumn" };
        obj.insert("season".to_string(), json!(season));
        let biome = obj.get("biome").and_then(|v| v.as_str()).unwrap_or("mediterranean").to_string();
        let (_, range, food_base, water_base, _) = BIOMES.iter().find(|(k, ..)| *k == biome).copied().unwrap_or(BIOMES[3]);
        let tmin = range[0];
        let tmax = range[1];
        let tmid = (tmin + tmax) / 2.0;
        let tamp = ((tmax - tmin) / 3.0).min(15.0);
        obj.insert("temperature".to_string(), json!((tmid + tamp * ((day_of_year as f64 - 80.0) / 365.0 * std::f64::consts::TAU).sin()).round()));
        let season_multiplier = match season { "summer" => 1.3, "winter" => 0.4, "spring" => 0.9, _ => 1.1 };
        // Density-dependent depletion: this population's own size relative
        // to this biome's own carrying capacity (same `food * 500` figure
        // compute_resource_pressure already uses) gradually pushes
        // human_impact toward how crowded the band actually is, smoothed
        // (5%/tick) rather than snapping instantly so a single day's
        // fluctuation doesn't whiplash the food ceiling, and easing back
        // down (never a permanent scar) once the population later shrinks.
        // Previously this field was inserted once at simulation creation and
        // never written again, so it stayed exactly 0 for the entire run --
        // no crowding pressure on the food ceiling ever actually applied.
        let carrying_capacity = (food_base * 500.0).max(1.0);
        let impact_target = (population_size as f64 / carrying_capacity).min(2.0);
        let prev_human_impact = obj.get("human_impact").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let human_impact = prev_human_impact + (impact_target - prev_human_impact) * 0.05;
        obj.insert("human_impact".to_string(), json!(human_impact));
        let mut base_food = (food_base * season_multiplier - human_impact * 0.1).max(0.05);
        let mut base_water = water_base;
        let mut tech_food_floor: f64 = 0.05;
        if let Some(techs) = discovered_techs {
            if techs.contains("food_preservation") { tech_food_floor = 0.15; }
            if techs.contains("plant_cultivation") { tech_food_floor = tech_food_floor.max(0.18); }
            if techs.contains("animal_herding") { tech_food_floor = tech_food_floor.max(0.20); }
            if techs.contains("pottery") { tech_food_floor = tech_food_floor.max(0.22); }
        }
        obj.insert("natural_disaster".to_string(), Value::Null);

        // Organic weather transitions -- previously current_weather was set
        // once (to "clear") at simulation creation and never updated by
        // anything except a manual God Mode override, so 7 of the 8
        // documented weather types could never occur on their own, and the
        // weather-dependent bonuses below (as well as
        // natural_disaster_probability's own drought/blizzard checks) were
        // permanently dead in organic play.
        let remaining = obj.get("weather_days_remaining").and_then(Value::as_i64).unwrap_or(0);
        if remaining > 0 {
            obj.insert("weather_days_remaining".to_string(), json!(remaining - 1));
        } else {
            let (weather, intensity, duration) = roll_weather(&biome, season);
            let (move_mult, hp_delta, cold_risk, heat_risk, water_delta, food_delta) = weather_effects(weather, intensity);
            obj.insert("current_weather".to_string(), json!(weather));
            obj.insert("weather_intensity".to_string(), json!(intensity));
            obj.insert("weather_days_remaining".to_string(), json!(duration));
            obj.insert("weather_move_mult".to_string(), json!(move_mult));
            obj.insert("weather_hp_delta".to_string(), json!(hp_delta));
            obj.insert("weather_cold_risk".to_string(), json!(cold_risk));
            obj.insert("weather_heat_risk".to_string(), json!(heat_risk));
            obj.insert("_weather_water_delta".to_string(), json!(water_delta));
            obj.insert("_weather_food_delta".to_string(), json!(food_delta));
        }
        let weather_food_delta = obj.get("_weather_food_delta").and_then(Value::as_f64).unwrap_or(0.0);
        let weather_water_delta = obj.get("_weather_water_delta").and_then(Value::as_f64).unwrap_or(0.0);
        base_food += weather_food_delta;
        base_water += weather_water_delta;

        obj.insert("food_abundance".to_string(), json!(base_food.max(tech_food_floor).min(1.0)));
        obj.insert("water_abundance".to_string(), json!(base_water.clamp(0.02, 1.0)));
    }
}

const WEATHER_TYPES: &[&str] = &["clear", "rain", "heavy_rain", "snow", "blizzard", "storm", "heat_wave", "drought"];

/// Biome/season-appropriate weather candidates and their relative weights --
/// snow/blizzard only ever show up somewhere cold (and preferentially in
/// winter), heat_wave/drought only somewhere hot (and preferentially in
/// summer), rain-heavy types favor wet biomes, and "clear" is always
/// possible everywhere as the common baseline.
fn weather_candidates(biome: &str, season: &str) -> Vec<(&'static str, f64)> {
    let cold_biome = matches!(biome, "tundra" | "boreal_forest" | "mountain");
    let hot_biome = matches!(biome, "desert" | "tropical_savanna");
    let wet_biome = matches!(biome, "tropical_rainforest" | "coastal" | "tropical_savanna");
    let temperate_biome = matches!(biome, "temperate_forest" | "grassland" | "mediterranean");

    let mut candidates: Vec<(&'static str, f64)> = vec![("clear", 3.0), ("storm", 0.4)];
    if wet_biome {
        candidates.push(("rain", 2.5));
        candidates.push(("heavy_rain", 1.2));
    } else {
        candidates.push(("rain", 1.0));
    }
    if season == "winter" && (cold_biome || temperate_biome) {
        candidates.push(("snow", if cold_biome { 2.0 } else { 0.5 }));
        if cold_biome {
            candidates.push(("blizzard", 1.0));
        }
    }
    if season == "summer" && (hot_biome || temperate_biome) {
        candidates.push(("heat_wave", if hot_biome { 1.5 } else { 0.4 }));
        candidates.push(("drought", if hot_biome { 1.2 } else { 0.3 }));
    } else if hot_biome {
        candidates.push(("drought", 0.6));
    }
    candidates
}

/// Picks the next weather type/intensity/duration once the current spell has
/// run its course.
fn roll_weather(biome: &str, season: &str) -> (&'static str, f64, i64) {
    let candidates = weather_candidates(biome, season);
    let total: f64 = candidates.iter().map(|(_, w)| w).sum();
    let mut pick = rand::random::<f64>() * total;
    let mut chosen = "clear";
    for (name, w) in &candidates {
        if pick < *w {
            chosen = name;
            break;
        }
        pick -= w;
    }
    debug_assert!(WEATHER_TYPES.contains(&chosen), "roll_weather picked {chosen}, not one of the 8 declared WEATHER_TYPES");
    let intensity = 0.3 + rand::random::<f64>() * 0.7;
    let duration = if chosen == "clear" { 3 + (rand::random::<f64>() * 5.0) as i64 } else { 1 + (rand::random::<f64>() * 4.0) as i64 };
    (chosen, intensity, duration)
}

/// Movement-speed multiplier, direct HP delta/tick, cold/heat exposure
/// flags, and water/food abundance deltas for a given (weather, intensity)
/// pair -- scaled so a single weather spell never dominates mortality on its
/// own; it's a meaningful nudge, not a scripted kill event (natural
/// disasters remain the mechanism for that).
fn weather_effects(weather: &str, intensity: f64) -> (f64, f64, bool, bool, f64, f64) {
    match weather {
        "clear" => (1.0, 0.0, false, false, 0.0, 0.0),
        "rain" => (0.9, 0.0, false, false, 0.05 * intensity, 0.02 * intensity),
        "heavy_rain" => (0.7, -0.002 * intensity, false, false, 0.1 * intensity, 0.0),
        "snow" => (0.6, -0.003 * intensity, true, false, 0.0, -0.05 * intensity),
        "blizzard" => (0.3, -0.01 * intensity, true, false, 0.0, -0.1 * intensity),
        "storm" => (0.5, -0.005 * intensity, false, false, 0.02 * intensity, -0.02 * intensity),
        "heat_wave" => (0.8, -0.004 * intensity, false, true, -0.1 * intensity, -0.03 * intensity),
        "drought" => (1.0, 0.0, false, false, -0.15 * intensity, -0.15 * intensity),
        _ => (1.0, 0.0, false, false, 0.0, 0.0),
    }
}

const BIOME_DISASTER_RISK: &[(&str, f64)] = &[
    ("mediterranean", 0.0003),
    ("coastal", 0.0005),
    ("tropical_rainforest", 0.0006),
    ("tropical_savanna", 0.0005),
    ("temperate_forest", 0.0003),
    ("boreal_forest", 0.0002),
    ("tundra", 0.0002),
    ("mountain", 0.0007),
    ("grassland", 0.0003),
    ("desert", 0.0004),
];

/// Base daily probability of a natural disaster striking, given the current
/// biome/weather. Distinct from god-mode-triggered disasters -- this is the
/// organic, unprompted background risk every simulation runs under.
pub fn natural_disaster_probability(world_state: &Value) -> f64 {
    if world_state.get("quarantine_mode").and_then(Value::as_bool).unwrap_or(false) {
        return 0.0;
    }
    let biome = world_state.get("biome").and_then(Value::as_str).unwrap_or("mediterranean");
    let mut base = BIOME_DISASTER_RISK.iter().find(|(b, _)| *b == biome).map(|(_, r)| *r).unwrap_or(0.0003);
    let weather = world_state.get("current_weather").and_then(Value::as_str).unwrap_or("clear");
    if weather == "drought" {
        base *= 2.0;
    }
    if weather == "blizzard" {
        base *= 1.5;
    }
    base
}

/// Picks a disaster type appropriate to the current biome/weather -- some
/// candidates are only possible in certain conditions, plus a generic
/// fallback (drought_event) that's always available.
pub fn pick_natural_disaster(world_state: &Value) -> (String, f64) {
    let biome = world_state.get("biome").and_then(Value::as_str).unwrap_or("mediterranean");
    let weather = world_state.get("current_weather").and_then(Value::as_str).unwrap_or("clear");
    let mut candidates: Vec<(&str, f64)> = Vec::new();
    if ["mountain", "coastal", "temperate_forest"].contains(&biome) {
        candidates.push(("earthquake", 0.08));
    }
    if ["coastal", "tropical_rainforest", "tropical_savanna"].contains(&biome) || weather == "heavy_rain" {
        candidates.push(("flood", 0.07));
    }
    if ["desert", "grassland", "mediterranean"].contains(&biome) || weather == "drought" {
        candidates.push(("wildfire", 0.05));
    }
    if ["tundra", "boreal_forest"].contains(&biome) || weather == "blizzard" {
        candidates.push(("blizzard_disaster", 0.06));
    }
    candidates.push(("drought_event", 0.03));
    let (disaster_type, mortality_factor) = candidates[rand::random::<usize>() % candidates.len()];
    (disaster_type.to_string(), mortality_factor)
}

/// Applies a natural disaster's mortality roll to every living individual,
/// removes the dead from their group's member_ids, and raises disaster fear
/// (and flood-specific water fear) in every survivor -- natural disasters are
/// regional events, so witnessing doesn't depend on distance.
pub fn process_disaster(
    disaster_type: &str,
    mortality_factor: f64,
    population: &mut [crate::state::Individual],
    groups: &mut [Value],
    day: i32,
) -> Vec<Value> {
    let mut deaths = 0_i64;
    let mut dead_ids: Vec<String> = Vec::new();
    let mut dead_founder_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ind in population.iter_mut() {
        if ind.is_dead {
            continue;
        }
        // Unlike compute_daily_death_risk (mortality.rs), this used to apply the
        // exact same flat mortality_factor to every individual regardless of
        // is_founder or phenotype -- founders got no protection here even though
        // they're deliberately protected everywhere else in the mortality model
        // (0.5x on the daily background risk, 0.4x on predator/disease risk).
        // Skipped for near-certain-death disasters (mortality_factor >= 0.99) so
        // a "wipes out everyone" event still means literally everyone, matching
        // the existing regression test for that case.
        let mut individual_risk = mortality_factor;
        if mortality_factor < 0.99 {
            let toughness = (ind.phenotype.endurance + ind.phenotype.physical_strength) / 2.0;
            let resilience = (ind.phenotype.stress_resilience + ind.phenotype.health_resilience) / 2.0;
            individual_risk *= 1.0 - (toughness - 0.5) * 0.3;
            individual_risk *= 1.0 - (resilience - 0.5) * 0.2;
            if ind.is_founder {
                individual_risk *= 0.5;
            }
            individual_risk = individual_risk.clamp(0.0, 1.0);
        }
        if rand::random::<f64>() < individual_risk {
            ind.is_dead = true;
            ind.alive = false;
            ind.death_day = Some(day);
            ind.extra.insert("death_cause".to_string(), json!(disaster_type));
            dead_ids.push(ind.id.clone());
            if ind.is_founder {
                dead_founder_ids.insert(ind.id.clone());
            }
            deaths += 1;
        }
    }
    if !dead_ids.is_empty() {
        for group in groups.iter_mut() {
            if let Some(members) = group.get_mut("member_ids").and_then(Value::as_array_mut) {
                members.retain(|v| !dead_ids.iter().any(|id| v.as_str() == Some(id.as_str())));
            }
        }
    }

    let mut events = Vec::new();
    if deaths > 0 {
        events.push(json!({
            "type": "disaster",
            "disaster_type": disaster_type,
            "deaths": deaths,
            "mortality_factor": mortality_factor,
            "day": day,
        }));
        // The aggregate "disaster" event above only carries a death *count*,
        // never who died -- unlike every other mortality path (ordinary
        // mortality, birth complications, infection), which pushes an
        // individual "death" event per victim. Without this, a disaster
        // death is correctly reflected in the individuals table (so the
        // population panel and top-bar birth/death counts are right) but
        // never appears in the event log at all.
        for id in &dead_ids {
            events.push(json!({
                "type": "death",
                "individual_id": id,
                "cause": disaster_type,
                "day": day,
                "importance": "medium",
                "is_founder": dead_founder_ids.contains(id),
            }));
        }
    }

    for ind in population.iter_mut() {
        if ind.is_dead {
            continue;
        }
        let mut fears = ind.extra.get("_fears").cloned().unwrap_or_else(|| json!({}));
        let current = fears.get("disaster").and_then(Value::as_f64).unwrap_or(0.0);
        if let Some(obj) = fears.as_object_mut() {
            obj.insert("disaster".to_string(), json!((current * 0.6 + 0.5_f64).min(1.0)));
        }
        ind.extra.insert("_fears".to_string(), fears);
        if disaster_type == "flood" {
            let water_fear = ind.extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
            ind.extra.insert("_waterFear".to_string(), json!((water_fear + 0.3).min(1.0)));
            // Standing floodwater takes a few days to recede -- `_inWater`
            // stays true for that stretch, which is what actually gives the
            // (already-implemented and tested) mortality::compute_daily_death_risk
            // drowning bonus, and DeathCause::Drowning attribution, anything
            // to react to. Previously nothing in production ever set this flag.
            ind.extra.insert("_inWater".to_string(), json!(true));
            ind.extra.insert("_wasInWater".to_string(), json!(true));
            ind.extra.insert("_inWaterDaysRemaining".to_string(), json!(FLOOD_WATER_DAYS));
        }
    }

    events
}

const FLOOD_WATER_DAYS: i64 = 3;

/// Per-tick water-state upkeep for a single living individual: counts down
/// standing floodwater (see `process_disaster`'s "flood" branch) back to dry
/// land, grants `_waterExperience` for having survived it (AGENTS.md: gained
/// "while in water"), and decays `_waterFear` at the documented 0.0005/day
/// rate. Call once per living individual per tick.
pub fn update_water_state(individual: &mut crate::state::Individual) {
    let remaining = individual.extra.get("_inWaterDaysRemaining").and_then(Value::as_i64).unwrap_or(0);
    if remaining > 0 {
        individual.extra.insert("_inWater".to_string(), json!(true));
        let next = remaining - 1;
        individual.extra.insert("_inWaterDaysRemaining".to_string(), json!(next));
        if next == 0 {
            let exp = individual.extra.get("_waterExperience").and_then(Value::as_f64).unwrap_or(0.0);
            individual.extra.insert("_waterExperience".to_string(), json!((exp + 0.15).min(1.0)));
        }
    } else {
        individual.extra.insert("_inWater".to_string(), json!(false));
    }
    let fear = individual.extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
    if fear > 0.0 {
        individual.extra.insert("_waterFear".to_string(), json!((fear - 0.0005).max(0.0)));
    }
}

pub fn compute_resource_pressure(world_state: &Value, population_size: usize) -> Value {
    let food = world_state.get("food_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let water = world_state.get("water_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let carrying_capacity = food * 500.0;
    let pressure = population_size as f64 / carrying_capacity.max(1.0);
    json!({
        "food_pressure": pressure.min(1.0),
        "water_pressure": (population_size as f64 / (water * 1000.0).max(1.0)).min(1.0),
        "carrying_capacity": carrying_capacity.round() as i64,
        "overpopulated": pressure > 1.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn anatolia_starting_region_is_on_land() {
        assert!(is_on_land(38.0, 35.0), "Anatolia (the sim's starting region) must be on land");
    }

    // ── organic weather transitions (V-11 regression) ───────────────────

    #[test]
    fn weather_can_organically_become_something_other_than_clear() {
        // Previously current_weather was set once (to "clear") at creation
        // and never updated by anything except a manual God Mode override --
        // this must now change on its own given enough days.
        let mut world = create_world_state(38.0, 35.0);
        let mut saw_non_clear = false;
        for day in 0..2000 {
            update_world_state(&mut world, day, None, 20);
            if world["current_weather"] != "clear" {
                saw_non_clear = true;
                break;
            }
        }
        assert!(saw_non_clear, "weather should organically change given enough days");
    }

    // ── density-dependent human_impact (previously stuck at 0 forever) ────

    #[test]
    fn a_crowded_population_gradually_raises_human_impact_above_zero() {
        // mediterranean's food_base (0.75) gives a carrying_capacity of
        // 0.75*500 = 375 -- a population of 400 sits just over it.
        let mut world = create_world_state(38.0, 35.0);
        assert_eq!(world["human_impact"], json!(0));
        for day in 0..200 {
            update_world_state(&mut world, day, None, 400);
        }
        assert!(world["human_impact"].as_f64().unwrap_or(0.0) > 0.0, "a population above carrying capacity should raise human_impact above its initial zero");
    }

    #[test]
    fn an_empty_population_lets_human_impact_settle_back_toward_zero() {
        let mut world = create_world_state(38.0, 35.0);
        for day in 0..200 {
            update_world_state(&mut world, day, None, 400);
        }
        let crowded_impact = world["human_impact"].as_f64().unwrap_or(0.0);
        assert!(crowded_impact > 0.0);
        for day in 200..800 {
            update_world_state(&mut world, day, None, 0);
        }
        let settled_impact = world["human_impact"].as_f64().unwrap_or(0.0);
        assert!(settled_impact < crowded_impact, "human_impact should ease back down once the population that caused it is gone");
        assert!(settled_impact < 0.01, "human_impact should settle close to zero, got {settled_impact}");
    }

    #[test]
    fn human_impact_never_snaps_instantly_to_its_target() {
        // The 5%/tick smoothing means one single day at a much higher
        // population size shouldn't already put human_impact near its
        // asymptotic target.
        let mut world = create_world_state(38.0, 35.0);
        update_world_state(&mut world, 0, None, 1000);
        let after_one_day = world["human_impact"].as_f64().unwrap_or(0.0);
        assert!(after_one_day > 0.0 && after_one_day < 0.2, "a single tick should only nudge human_impact a little, got {after_one_day}");
    }

    #[test]
    fn a_higher_human_impact_lowers_food_abundance() {
        let mut low = create_world_state(38.0, 35.0);
        let mut high = create_world_state(38.0, 35.0);
        for day in 0..300 {
            update_world_state(&mut low, day, None, 5);
            update_world_state(&mut high, day, None, 2000);
        }
        assert!(
            high["food_abundance"].as_f64().unwrap() < low["food_abundance"].as_f64().unwrap(),
            "a heavily overcrowded population should end up with lower food_abundance than a sparse one"
        );
    }

    #[test]
    fn all_eight_weather_types_are_reachable_across_biomes_and_seasons() {
        let mut seen: HashSet<String> = HashSet::new();
        for biome in ["tundra", "desert", "tropical_rainforest", "mediterranean"] {
            let mut world = json!({ "biome": biome, "weather_days_remaining": 0 });
            for day in 0..3650 {
                update_world_state(&mut world, day, None, 20);
                seen.insert(world["current_weather"].as_str().unwrap().to_string());
            }
        }
        for w in WEATHER_TYPES {
            assert!(seen.contains(*w), "expected weather type {w} to be reachable, saw: {seen:?}");
        }
    }

    #[test]
    fn weather_days_remaining_counts_down_before_rerolling() {
        let mut world = json!({ "biome": "mediterranean", "current_weather": "rain", "weather_days_remaining": 3 });
        update_world_state(&mut world, 0, None, 20);
        assert_eq!(world["weather_days_remaining"], 2);
        assert_eq!(world["current_weather"], "rain", "weather must not reroll before its remaining days elapse");
    }

    #[test]
    fn snow_and_blizzard_never_occur_in_a_hot_desert_biome() {
        let mut world = json!({ "biome": "desert", "weather_days_remaining": 0 });
        for day in 0..3650 {
            update_world_state(&mut world, day, None, 20);
            let w = world["current_weather"].as_str().unwrap();
            assert!(w != "snow" && w != "blizzard", "desert should never roll {w}");
        }
    }

    #[test]
    fn drought_and_heat_wave_never_occur_in_a_frozen_tundra_biome() {
        let mut world = json!({ "biome": "tundra", "weather_days_remaining": 0 });
        for day in 0..3650 {
            update_world_state(&mut world, day, None, 20);
            let w = world["current_weather"].as_str().unwrap();
            assert!(w != "drought" && w != "heat_wave", "tundra should never roll {w}");
        }
    }

    #[test]
    fn weather_derived_food_and_water_deltas_actually_move_abundance() {
        // _weather_food_delta/_weather_water_delta were previously computed
        // fields nothing anywhere ever read.
        let mut drought_world = json!({ "biome": "desert", "weather_days_remaining": 0 });
        let mut saw_reduced_food = false;
        for day in 0..3650 {
            update_world_state(&mut drought_world, day, None, 20);
            if drought_world["current_weather"] == "drought" {
                // Compare against a clear-weather baseline for the same day/biome.
                let mut clear_world = json!({ "biome": "desert", "current_weather": "clear", "weather_days_remaining": 999 });
                update_world_state(&mut clear_world, day, None, 20);
                if drought_world["food_abundance"].as_f64().unwrap() < clear_world["food_abundance"].as_f64().unwrap() {
                    saw_reduced_food = true;
                    break;
                }
            }
        }
        assert!(saw_reduced_food, "a drought should measurably reduce food_abundance relative to clear weather");
    }

    #[test]
    fn mid_atlantic_and_mid_pacific_are_not_on_land() {
        assert!(!is_on_land(20.0, -40.0), "mid-Atlantic must not be classified as land");
        assert!(!is_on_land(0.0, -150.0), "mid-Pacific must not be classified as land");
    }

    #[test]
    fn longitude_wraps_correctly_across_the_antimeridian() {
        assert_eq!(is_on_land(38.0, 35.0), is_on_land(38.0, 35.0 + 360.0));
    }

    // Regression: a coarse continent-bounding-box approximation used to treat
    // these inland seas and lakes as "land" (they sit inside the
    // Europe/Asia/N.America boxes), so migrating individuals ended up
    // visually stranded on open water on the client's Globe map.
    #[test]
    fn inland_seas_and_major_lakes_are_not_on_land() {
        assert!(!is_on_land(35.5, 18.0), "Mediterranean Sea must not be classified as land");
        assert!(!is_on_land(43.4, 34.5), "Black Sea must not be classified as land");
        assert!(!is_on_land(41.5, 50.5), "Caspian Sea must not be classified as land");
        assert!(!is_on_land(27.0, 51.0), "Persian Gulf must not be classified as land");
        assert!(!is_on_land(20.0, 38.0), "Red Sea must not be classified as land");
        assert!(!is_on_land(58.0, 19.0), "Baltic Sea must not be classified as land");
        assert!(!is_on_land(60.0, -85.0), "Hudson Bay must not be classified as land");
        assert!(!is_on_land(47.7, -87.5), "Lake Superior must not be classified as land");
        assert!(!is_on_land(53.5, 108.0), "Lake Baikal must not be classified as land");
    }

    // ── H-01 regression — process_disaster() death tracking ────────────

    use crate::state::Individual;

    fn make_ind(id: &str) -> Individual {
        Individual { id: id.to_string(), alive: true, is_dead: false, ..Default::default() }
    }

    #[test]
    fn disaster_deaths_are_marked_dead_with_the_disaster_as_cause() {
        let mut population = vec![make_ind("i1")];
        // mortality_factor 1.0 -> everyone dies.
        let events = process_disaster("wildfire", 1.0, &mut population, &mut [], 0);
        assert!(population[0].is_dead);
        assert!(!population[0].alive);
        assert_eq!(population[0].death_day, Some(0));
        assert_eq!(population[0].extra["death_cause"], "wildfire");
        assert_eq!(events[0]["type"], "disaster");
        assert_eq!(events[0]["deaths"], 1);
    }

    #[test]
    fn disaster_deaths_remove_the_individual_from_their_group_member_ids() {
        let mut population = vec![make_ind("i1")];
        let mut groups = vec![json!({ "id": "grp-1", "member_ids": ["i1"] })];
        process_disaster("flood", 1.0, &mut population, &mut groups, 0);
        assert!(!groups[0]["member_ids"].as_array().unwrap().iter().any(|v| v == "i1"));
    }

    #[test]
    fn bug12_regression_all_individuals_die_when_mortality_factor_is_one() {
        let mut population: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"))).collect();
        let events = process_disaster("wildfire", 1.0, &mut population, &mut [], 0);
        assert!(population.iter().all(|i| i.is_dead));
        assert_eq!(events[0]["deaths"], 5);
    }

    #[test]
    fn disaster_with_zero_mortality_factor_kills_nobody() {
        let mut population = vec![make_ind("i1")];
        let events = process_disaster("drought_event", 0.0, &mut population, &mut [], 0);
        assert!(!population[0].is_dead);
        assert!(events.is_empty());
    }

    #[test]
    fn founders_are_meaningfully_more_likely_to_survive_a_disaster_than_non_founders() {
        // Regression test: process_disaster used to apply the exact same flat
        // mortality_factor to everyone, unlike the daily background risk
        // (mortality.rs), which deliberately halves risk for founders. Over
        // repeated earthquakes that discrepancy alone was enough to kill
        // founders within a decade despite them being "protected" everywhere
        // else in the mortality model.
        const TRIALS: usize = 4000;
        let mut founder_deaths = 0;
        let mut non_founder_deaths = 0;
        for i in 0..TRIALS {
            let mut population = vec![
                Individual { id: format!("f{i}"), alive: true, is_dead: false, is_founder: true, ..Default::default() },
                Individual { id: format!("n{i}"), alive: true, is_dead: false, is_founder: false, ..Default::default() },
            ];
            process_disaster("earthquake", 0.5, &mut population, &mut [], 0);
            if population[0].is_dead {
                founder_deaths += 1;
            }
            if population[1].is_dead {
                non_founder_deaths += 1;
            }
        }
        assert!(
            founder_deaths < non_founder_deaths,
            "founders ({founder_deaths}/{TRIALS}) should die less often than non-founders ({non_founder_deaths}/{TRIALS}) in the same disaster"
        );
    }

    #[test]
    fn death_event_carries_is_founder_matching_the_actual_victim() {
        // The frontend plays a distinct founder-death alarm keyed off
        // data.is_founder -- this must actually reflect who died, not just
        // be present as a field.
        let mut population = vec![
            Individual { id: "founder-1".to_string(), alive: true, is_dead: false, is_founder: true, ..Default::default() },
            Individual { id: "child-1".to_string(), alive: true, is_dead: false, is_founder: false, ..Default::default() },
        ];
        let events = process_disaster("earthquake", 1.0, &mut population, &mut [], 5);
        let deaths: Vec<&Value> = events.iter().filter(|e| e["type"] == "death").collect();
        assert_eq!(deaths.len(), 2, "mortality_factor 1.0 must kill everyone");
        let founder_event = deaths.iter().find(|e| e["individual_id"] == "founder-1").unwrap();
        let child_event = deaths.iter().find(|e| e["individual_id"] == "child-1").unwrap();
        assert_eq!(founder_event["is_founder"], true);
        assert_eq!(child_event["is_founder"], false);
    }

    #[test]
    fn disaster_raises_disaster_fear_in_every_survivor() {
        let mut population = vec![make_ind("i1")];
        process_disaster("wildfire", 0.0, &mut population, &mut [], 0);
        let fear = population[0].extra["_fears"]["disaster"].as_f64().unwrap();
        assert!((fear - 0.5).abs() < 1e-9);
    }

    #[test]
    fn flood_disasters_additionally_raise_water_fear() {
        let mut population = vec![make_ind("i1")];
        process_disaster("flood", 0.0, &mut population, &mut [], 0);
        assert!((population[0].extra["_waterFear"].as_f64().unwrap() - 0.3).abs() < 1e-9);
        // A non-flood disaster must not touch water fear at all.
        let mut population2 = vec![make_ind("i1")];
        process_disaster("wildfire", 0.0, &mut population2, &mut [], 0);
        assert!(population2[0].extra.get("_waterFear").is_none());
    }

    // ── natural disaster selection ──────────────────────────────────────

    #[test]
    fn quarantine_mode_suppresses_all_disaster_probability() {
        let world = json!({ "quarantine_mode": true, "biome": "mountain" });
        assert_eq!(natural_disaster_probability(&world), 0.0);
    }

    #[test]
    fn drought_weather_doubles_disaster_probability() {
        let calm = json!({ "biome": "desert", "current_weather": "clear" });
        let drought = json!({ "biome": "desert", "current_weather": "drought" });
        assert!((natural_disaster_probability(&drought) - natural_disaster_probability(&calm) * 2.0).abs() < 1e-12);
    }

    #[test]
    fn earthquake_is_never_picked_outside_its_eligible_biomes() {
        // Earthquake only applies to mountain/coastal/temperate_forest; a desert
        // with clear weather should never roll it (fallback is drought_event).
        let world = json!({ "biome": "desert", "current_weather": "clear" });
        for _ in 0..1000 {
            let (disaster_type, _) = pick_natural_disaster(&world);
            assert_ne!(disaster_type, "earthquake");
        }
    }

    #[test]
    fn drought_event_is_always_a_possible_fallback() {
        // Every biome/weather combination includes drought_event as a candidate,
        // so over enough trials it must appear even when other types are also eligible.
        let world = json!({ "biome": "mountain", "current_weather": "clear" });
        let mut seen_fallback = false;
        for _ in 0..2000 {
            let (disaster_type, _) = pick_natural_disaster(&world);
            if disaster_type == "drought_event" {
                seen_fallback = true;
                break;
            }
        }
        assert!(seen_fallback);
    }
}
