//! Ad-hoc manual smoke test, not part of the test suite: builds a fresh
//! two-founder simulation exactly like routes.rs::create_simulation does,
//! then drives advance_one_day directly (in-process, no HTTP/DB/auth --
//! those are plumbing sim-core doesn't own) for many simulated years,
//! watching for panics, extinction, NaN/Infinity in computed stats, and
//! basic signs of life (reproduction, tech/belief/art discovery).
//!
//! Run with: cargo run --example smoke_run -p sim-core --release -- [years]

use sim_core::{advance_one_day, compute_genetic_diversity, create_founder, Individual, SimulationState, WorldState};
use std::panic;

fn founder(sex: &str, age_years: i64, x: f64, y: f64) -> Individual {
    let mut ind = create_founder(&serde_json::json!({ "sex": sex, "ageYears": age_years, "x": x, "y": y }));
    ind.simulation_id = Some("smoke-test".to_string());
    ind
}

fn main() {
    let years: i32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(20);
    let days = years * 365;

    let latitude = 40.0;
    let longitude = 30.0;
    let world_value = sim_core::create_world_state(latitude, longitude);
    let world_state: WorldState = serde_json::from_value(world_value).unwrap_or_default();

    let mut state = SimulationState {
        current_day: 0,
        current_year: 0,
        status: Some("running".to_string()),
        speed_multiplier: Some(1),
        world_state,
        start_latitude: Some(latitude),
        start_longitude: Some(longitude),
        discovered_techs: vec!["foraging".to_string(), "stone_tools".to_string()],
        individuals: vec![founder("male", 22, longitude, latitude), founder("female", 20, longitude + 0.1, latitude)],
        ..Default::default()
    };
    state.total_ever_born = state.individuals.len() as i32;

    println!("=== smoke_run: {years} sim-years, biome={:?} ===", state.world_state.biome);

    let mut panics: Vec<(i32, String)> = Vec::new();
    for day in 0..days {
        match panic::catch_unwind(panic::AssertUnwindSafe(|| advance_one_day(&mut state))) {
            Ok(_) => {}
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic payload".to_string());
                eprintln!("!! PANIC on day {day}: {msg}");
                panics.push((day, msg));
                state.current_day += 1;
            }
        }

        if day % 365 == 0 || day == days - 1 {
            let alive: Vec<&Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
            let avg_age = if alive.is_empty() {
                0.0
            } else {
                alive.iter().map(|i| (state.current_day - i.birth_day) as f64 / 365.0).sum::<f64>() / alive.len() as f64
            };
            let weather = state.world_state.extra.get("current_weather").and_then(|v| v.as_str()).unwrap_or("?");
            let season = state.world_state.season.as_deref().unwrap_or("?");
            println!(
                "day {:5} (yr {:3}) | pop {:4} | avg_age {:5.1} | techs {:2} | beliefs {} | arts {} | groups {} | season {:7} | weather {}",
                state.current_day,
                state.current_year,
                alive.len(),
                avg_age,
                state.discovered_techs.len(),
                state.discovered_beliefs.len(),
                state.discovered_arts.len(),
                state.groups.len(),
                season,
                weather,
            );
            if alive.is_empty() {
                println!("!! POPULATION EXTINCT at day {day}");
                break;
            }
        }
    }

    let alive: Vec<&Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    println!("\n=== FINAL ===");
    println!("day {} / year {}", state.current_day, state.current_year);
    println!("population: {} alive / {} ever born", alive.len(), state.total_ever_born);
    println!("technologies ({}): {:?}", state.discovered_techs.len(), state.discovered_techs);
    println!("beliefs discovered: {}", state.discovered_beliefs.len());
    println!("arts ({}): {:?}", state.discovered_arts.len(), state.discovered_arts);
    println!("groups: {}", state.groups.len());
    println!("max language stage: {}", state.individuals.iter().map(|i| i.language.stage).max().unwrap_or(0));
    println!("civilization_name: {:?}", state.civilization_name);

    for g in &state.groups {
        let member_count = g.get("member_ids").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        println!("group {:?}: member_ids.len()={member_count}", g.get("id"));
    }
    for i in state.individuals.iter().filter(|i| i.alive && !i.is_dead).take(10) {
        println!(
            "  ind {} sex={} group_id={:?} foxp2_expr={:.3} lang_capacity={:.3} stage={} generation={:?}",
            &i.id[..8.min(i.id.len())],
            i.sex,
            i.group_id,
            i.language.foxp2_expression,
            i.phenotype.language_capacity,
            i.language.stage,
            i.generation,
        );
    }

    let deaths = state.individuals.iter().filter(|i| i.is_dead).count();
    let mut death_causes: std::collections::HashMap<String, i32> = Default::default();
    for i in state.individuals.iter().filter(|i| i.is_dead) {
        let cause = i.extra.get("death_cause").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        *death_causes.entry(cause).or_insert(0) += 1;
    }
    println!("deaths: {deaths} -- by cause: {death_causes:?}");

    let alive_refs: Vec<&Individual> = alive.clone();
    let gd = compute_genetic_diversity(&alive_refs);
    println!("genetic diversity: {gd}");

    let mut failures: Vec<String> = Vec::new();
    if !panics.is_empty() {
        failures.push(format!("{} panics occurred (first at day {})", panics.len(), panics[0].0));
    }
    if alive.is_empty() {
        failures.push("population went extinct".to_string());
    }
    if state.total_ever_born <= 2 {
        failures.push("no reproduction ever happened (total_ever_born never grew past the 2 founders)".to_string());
    }
    if let Some(obj) = gd.as_object() {
        for (k, v) in obj {
            if let Some(f) = v.as_f64() {
                if f.is_nan() || f.is_infinite() {
                    failures.push(format!("genetic_diversity.{k} is {f}"));
                }
            }
        }
    }
    for i in &alive {
        if i.phenotype.fluid_intelligence.is_nan() || i.health.hp.is_nan() || i.psychology.wellbeing.is_nan() {
            failures.push(format!("individual {} has a NaN core stat", i.id));
            break;
        }
    }

    if failures.is_empty() {
        println!("\nRESULT: PASS");
    } else {
        println!("\nRESULT: FAIL -- {failures:?}");
        std::process::exit(1);
    }
}
