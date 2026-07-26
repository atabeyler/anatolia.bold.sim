//! Ad-hoc probe (not part of the test suite): runs many independent
//! two-founder simulations and reports how often/when each founder dies,
//! from what cause, and what their health metrics looked like in the days
//! leading up to death -- to investigate a report that founders die too
//! early in practice.
//!
//! Run with: cargo run --example founder_mortality_probe -p sim-core --release -- [trials] [years]

use sim_core::{advance_one_day, create_founder, Individual, SimulationState, WorldState};

fn founder(sex: &str, age_years: i64, x: f64, y: f64) -> Individual {
    let mut ind = create_founder(&serde_json::json!({ "sex": sex, "ageYears": age_years, "x": x, "y": y }));
    ind.simulation_id = Some("mortality-probe".to_string());
    ind
}

fn main() {
    let trials: i32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let years: i32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    let days = years * 365;

    let mut death_days: Vec<i32> = Vec::new();
    let mut death_causes: std::collections::HashMap<String, i32> = Default::default();
    let mut both_dead_within_year1 = 0;
    let mut both_dead_within_year5 = 0;
    let mut at_least_one_dead_by_end = 0;
    let mut lowest_hp_before_death_samples: Vec<(f64, f64, f64)> = Vec::new(); // (hp, calories, hydration) sampled the tick before death

    for trial in 0..trials {
        let latitude = 30.0 + (trial as f64 * 3.7) % 30.0;
        let longitude = 20.0 + (trial as f64 * 5.3) % 40.0;
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

        let founder_ids: Vec<String> = state.individuals.iter().map(|i| i.id.clone()).collect();
        let mut prev_health: std::collections::HashMap<String, (f64, f64, f64)> = Default::default();

        for day in 0..days {
            advance_one_day(&mut state);
            for id in &founder_ids {
                if let Some(ind) = state.individuals.iter().find(|i| &i.id == id) {
                    if ind.is_dead {
                        if let Some(&(hp, cal, hyd)) = prev_health.get(id) {
                            lowest_hp_before_death_samples.push((hp, cal, hyd));
                        }
                    } else {
                        prev_health.insert(id.clone(), (ind.health.hp, ind.health.calories, ind.health.hydration));
                    }
                }
            }
            let both_dead = state.individuals.iter().filter(|i| founder_ids.contains(&i.id)).all(|i| i.is_dead);
            if both_dead && day <= 365 {
                both_dead_within_year1 += 1;
            }
            if both_dead && day <= 365 * 5 {
                both_dead_within_year5 += 1;
            }
        }

        for id in &founder_ids {
            if let Some(ind) = state.individuals.iter().find(|i| &i.id == id) {
                if ind.is_dead {
                    let dday = ind.death_day.unwrap_or(-1);
                    death_days.push(dday);
                    let cause = ind.extra.get("death_cause").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                    *death_causes.entry(cause.clone()).or_insert(0) += 1;
                    if dday <= 400 {
                        println!(
                            "EARLY DEATH: trial {trial} day {dday} (~{:.1}mo) cause={cause} biome={:?} sex={}",
                            dday as f64 / 30.4,
                            state.world_state.biome,
                            ind.sex,
                        );
                    }
                }
            }
        }
        if state.individuals.iter().filter(|i| founder_ids.contains(&i.id)).any(|i| i.is_dead) {
            at_least_one_dead_by_end += 1;
        }
    }

    println!("=== founder_mortality_probe: {trials} trials x {years} years ===");
    println!("founder deaths recorded: {} (out of {} founders total)", death_days.len(), trials * 2);
    println!("at least one founder dead by end of run: {at_least_one_dead_by_end}/{trials} trials");
    println!("BOTH founders dead within year 1:  {both_dead_within_year1}/{trials} trials");
    println!("BOTH founders dead within year 5:  {both_dead_within_year5}/{trials} trials");
    if !death_days.is_empty() {
        death_days.sort();
        let avg = death_days.iter().sum::<i32>() as f64 / death_days.len() as f64;
        let median = death_days[death_days.len() / 2];
        println!("death day stats -- min: {} | median: {} | avg: {:.0} | max: {}", death_days[0], median, avg, death_days[death_days.len() - 1]);
        let under_1y = death_days.iter().filter(|&&d| d <= 365).count();
        let under_5y = death_days.iter().filter(|&&d| d <= 365 * 5).count();
        println!("individual founder deaths within 1yr: {under_1y}/{} | within 5yr: {under_5y}/{}", death_days.len(), death_days.len());
    }
    println!("death causes: {death_causes:?}");
    if !lowest_hp_before_death_samples.is_empty() {
        let n = lowest_hp_before_death_samples.len() as f64;
        let avg_hp = lowest_hp_before_death_samples.iter().map(|(h, _, _)| h).sum::<f64>() / n;
        let avg_cal = lowest_hp_before_death_samples.iter().map(|(_, c, _)| c).sum::<f64>() / n;
        let avg_hyd = lowest_hp_before_death_samples.iter().map(|(_, _, h)| h).sum::<f64>() / n;
        println!("avg health metrics on the tick BEFORE death (hp, calories, hydration): {avg_hp:.3}, {avg_cal:.3}, {avg_hyd:.3}");
    }
}
