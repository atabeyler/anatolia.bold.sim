use serde_json::Value;

use crate::epigenetics::compute_epigenetic_age;
use crate::microbiome::PATHOGEN_TYPES;
use crate::state::Individual;

use super::individual::get_age;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    Infection,
    /// Hypothermia/heatstroke -- the misadventure roll resolved to actively
    /// dangerous weather (`weather_cold_risk`/`weather_heat_risk`) at the
    /// moment of death. See `resolve_misadventure`.
    Exposure,
    /// A non-apex-predator animal encounter (bite/sting/goring) -- distinct
    /// from `Predator`, which represents an actual large-carnivore kill and
    /// is gated to biomes dangerous enough for that. Scales with the same
    /// `predator_risk` figure but reachable at any biome's danger level.
    WildlifeEncounter,
    /// The residual physical-mishap cause (falls, blunt injury, tool
    /// accidents) once weather and wildlife signals are ruled out -- kept
    /// deliberately narrow and honestly named rather than a catch-all.
    Injury,
    Starvation,
    Dehydration,
    BirthComplications,
    GeneticDisease,
    OldAge,
    Predator,
    Drowning,
    // Intergroup conflict deaths are attributed directly by social.rs (which
    // writes the "conflict" cause string straight into an individual's
    // death_cause, bypassing this enum entirely, since the intergroup
    // conflict mechanic lives outside the ordinary roll_death/determine_cause
    // path) -- there used to be a `Conflict` variant here too, but
    // determine_cause never actually constructed it, making it dead code.
}

/// Target annual mortality rates (prehistoric hunter-gatherer baseline):
/// 0-1y ~23.4%, 1-5y ~4.3%, 5-15y ~1.35%, 15-45y ~1.72%, 45-60y ~3.1%,
/// 60-75y ~10.5%, 75+ ~33%.
///
/// These are not a paraphrase or a rough recollection of Gurven & Kaplan
/// (2007) -- they're numerically derived directly from that paper's own
/// Table 2 (Siler mortality-hazard model parameters, PDR 33(2):321-365),
/// averaged across the five "traditional hunter-gatherer" populations it
/// reports (Hadza, Ache-forest, Hiwi, !Kung, Agta), by integrating each
/// population's own hazard function h(x) = a1*e^(-b1*x) + a2 + a3*e^(b3*x)
/// across each age band and converting the resulting per-band cumulative
/// survival probability to an annualized rate. The previous constants here
/// (0-1y ~8%, 1-5y ~3.7%, 5-15y ~1%, 15-45y ~1%, 45-60y ~2.5%, 60-75y ~8%,
/// 75+ ~20%) were never actually checked against the paper they cited --
/// every single band undershot the real figure, most severely 0-1y (the
/// coded value was less than half the real one) and 15-45y (the coded value
/// treated it identically to 5-15y, but the real data shows meaningfully
/// higher adult mortality once past childhood). This was found and fixed
/// specifically because `empirical_validation.rs`'s Monte Carlo harness had
/// been validating the simulation's *emergent* behavior against these
/// numbers as if they were solid ground truth -- a validation loop is only
/// as good as what it validates against, and this one had never actually
/// been checked. 75+ (integrated over the 75-85y span the paper's data
/// meaningfully covers) is the least certain of the seven: Siler-model
/// tails extrapolate poorly at extreme old age, and one population (Hiwi)
/// alone accounts for most of the tail's steepness.
/// Disease outbreaks, starvation, and disasters layer on top via multipliers.
pub fn compute_daily_death_risk(individual: &Individual, current_day: i32, environment: Option<&Value>) -> f64 {
    let chronological_age = get_age(individual, current_day);
    // Epigenetic age: accumulated stress/nutrition history can accelerate or
    // slow biological aging relative to the calendar.
    let epi_years = compute_epigenetic_age(individual, current_day);
    let age = if epi_years > 0.0 && epi_years.is_finite() { epi_years } else { chronological_age };

    let health = &individual.health;
    let phenotype = &individual.phenotype;
    let is_founder = individual.is_founder;

    // 5-15y and 15-45y now get their own constants (previously shared one
    // flat `age < 45.0` branch) -- the real data above no longer supports
    // treating them as identical; adult mortality past childhood runs
    // meaningfully higher than school-age childhood mortality does.
    let mut base_risk = if age < 1.0 {
        0.00064
    } else if age < 5.0 {
        0.000116
    } else if age < 15.0 {
        0.0000365
    } else if age < 45.0 {
        0.0000465
    } else if age < 60.0 {
        0.0000865
    } else if age < 75.0 {
        0.000302
    } else {
        0.00101
    };

    // These per-age-band figures were calibrated (see the doc comment
    // above) against a *total* mortality target that used to include
    // Predator/Injury/WildlifeEncounter/Exposure deaths resolved right here
    // by `determine_cause`'s own probability cascade. Those four causes now
    // come exclusively from `wounds::wound_collapse_cause` -- a real,
    // independent, physiologically-driven death check running alongside
    // this one, not a relabeling of a fraction of *this* probability.
    // Without this reduction, total mortality (this roll plus the new
    // wound-collapse one) would run well above the calibrated target.
    // NON_WOUND_CAUSE_SHARE estimates what fraction of overall mortality
    // this roll should still represent now that those four causes moved
    // elsewhere -- derived from determine_cause's old cascade weights
    // (misadventure_weight ~0.30 for adults, ~0.55-0.65 for children, plus
    // a smaller separate predator share), averaged and rounded to a single
    // constant rather than re-deriving a precise per-age-band figure, since
    // `empirical_validation.rs`'s Monte Carlo harness validates the
    // resulting *combined* rate empirically rather than trusting this
    // estimate exactly. Tune this (and wounds.rs's own infliction/severity
    // constants) together against that harness if the combined rate drifts
    // from the documented target.
    const NON_WOUND_CAUSE_SHARE: f64 = 0.65;
    base_risk *= NON_WOUND_CAUSE_SHARE;

    // Extinction guard: tiny bands receive outsized individual attention.
    // Exempts the 0-1y band specifically: a real infant's own mortality risk
    // doesn't fall just because the surrounding population is small -- the
    // documented ~23%/yr target above already accounts for real prehistoric
    // infant mortality, and a young colonizing population (the common case
    // this simulation spends most of its early life in, and exactly where
    // this guard would otherwise apply almost continuously) is not a reason
    // to discount that specific risk. `empirical_validation.rs`'s Monte
    // Carlo harness found this: the 0-1y band showed 0% emergent mortality
    // across 276 observed person-years before this exemption, against the
    // documented target, because this guard's up-to-4x discount compounded
    // with the immune_strength/resilience discounts below pushed the
    // effective rate an order of magnitude under target during exactly the
    // population-size regime a real simulation run spends the most time in.
    let alive_count = environment.and_then(|env| env.get("alive_count")).and_then(|v| v.as_f64()).unwrap_or(100.0);
    if alive_count < 25.0 && age >= 1.0 {
        base_risk *= (alive_count / 25.0).max(0.25);
    }

    // Thriving healthy individual: well-fed prime-years individuals get a
    // discount. Covers 5-45y, not just 15-45y: even though 5-15y and 15-45y
    // now carry their own distinct base_risk constants (see the doc comment
    // above -- the real Gurven & Kaplan data no longer supports treating
    // them as identical), both are still drawn from the same
    // low-background-mortality "childhood past infancy through prime
    // adulthood" segment of that data, so a well-fed, uninjured individual
    // gets the same real discount at 8 as at 25 -- it's the *base rate*
    // that differs between the two bands, not whether thriving health
    // should matter. Restricting this to 15+ (the original form) left the
    // 5-15y band with no way to reach its own target at all --
    // `empirical_validation.rs`'s Monte Carlo harness caught this as a
    // meaningful overshoot specific to 5-15y once the wound-collapse
    // mechanism (wounds.rs) was ruled out as the cause (it contributes zero
    // deaths to this band in practice).
    if (5.0..45.0).contains(&age) && health.hp > 0.85 && health.calories > 0.7 {
        base_risk *= 0.4;
    }

    if age >= phenotype.max_lifespan {
        base_risk += 0.03;
    }
    if health.hp < 0.2 {
        base_risk *= if is_founder { 1.8 } else { 3.0 };
    }
    if health.calories < 0.1 {
        base_risk *= if is_founder { 2.5 } else { 5.0 };
    }
    if health.hydration < 0.1 {
        base_risk *= if is_founder { 5.0 } else { 10.0 };
        // Elevated aldosterone (see hormones.rs) reflects the real renin-
        // angiotensin-aldosterone water/salt-retention response to low blood
        // volume -- a small, bounded discount on top of the severe base
        // multiplier above, the same "real hormonal adaptation partially
        // offsets a crisis" pattern as glucagon's own starvation discount
        // below.
        if individual.hormones.aldosterone > 0.5 {
            base_risk *= 1.0 - (individual.hormones.aldosterone - 0.5) * 0.15;
        }
    }
    // Erythropoietin (see hormones.rs) rises with low HP (a blood-loss/
    // anemia proxy) and reflects the real red-cell-production recovery
    // response -- a small, bounded discount once meaningfully elevated.
    if individual.hormones.epo > 0.6 {
        base_risk *= 1.0 - (individual.hormones.epo - 0.6) * 0.1;
    }

    base_risk *= 1.0 - phenotype.immune_strength * 0.3;

    let resilience = (phenotype.stress_resilience + phenotype.health_resilience) / 2.0;
    base_risk *= 1.0 - (resilience - 0.5) * 0.25;

    // Elevated PTH in a post-fertile female (see hormones.rs -- the real
    // estrogen-decline-driven bone-loss/osteoporosis pathway) carries a
    // small, bounded fracture-adjacent mortality risk, matching real elderly
    // osteoporotic-fracture epidemiology. Age-gated the same way the
    // senescence curve itself is (45y+), so it only ever applies where the
    // underlying PTH elevation is actually possible.
    if individual.sex == "female" && age >= 45.0 && individual.hormones.pth > 0.5 {
        base_risk += (individual.hormones.pth - 0.5) * 0.00004;
    }

    // Predator danger used to add a direct term here (a founder/toughness-
    // scaled contribution to whether *any* death happens this tick), with
    // `determine_cause` resolving the actual cause (Predator/WildlifeEncounter)
    // afterward. That resolution moved entirely to `wounds.rs`'s wound-
    // infliction/accumulation/collapse mechanism (see this file's own
    // `NON_WOUND_CAUSE_SHARE` doc comment), which already models predator
    // danger as its own physiological process -- keeping this term here too
    // double-counted the same danger twice: once deciding whether a death
    // happens at all, and again in wound accrual. Removed; predator/wildlife
    // risk to survival now flows exclusively through wounds.rs.

    if health.calories < 0.4 {
        base_risk *= 1.0 + (phenotype.metabolism - 0.5) * 0.2;
        // Elevated glucagon (see hormones.rs) reflects a genuine hormonal
        // fasting-adaptation response -- mobilized energy reserves -- so a
        // starving individual whose glucagon has actually ramped up gets a
        // small, bounded discount on top of the metabolism-driven term
        // above, rather than none at all.
        if individual.hormones.glucagon > 0.6 {
            base_risk *= 1.0 - (individual.hormones.glucagon - 0.6) * 0.2;
        }
    }

    if individual._in_water() {
        let water_skill = individual._water_experience().min(0.9) * 0.9;
        base_risk += 0.003 * (1.0 - water_skill);
    }

    // CLOT_01 (X-linked clotting-factor locus, genome.rs): a low value means
    // any injury bleeds longer before clotting -- a small, genetically-driven
    // bump to overall risk, distinct from (and additive to) the toughness-
    // based predator/trauma modeling above.
    let clotting_factor = phenotype.extra.get("clotting_factor").and_then(Value::as_f64).unwrap_or(0.7);
    if clotting_factor < 0.3 {
        base_risk *= 1.15;
    }

    if let Some(env) = environment {
        let env_mult = if is_founder { 0.4 } else { 1.0 };
        base_risk += env.get("disease_pressure").and_then(Value::as_f64).unwrap_or(0.0) * 0.0003 * env_mult;
    }

    // Sustained (>0.6) cortisol elevation -- chronic HPA-axis activation, see
    // hormones.rs -- carries a small, bounded extra mortality contribution,
    // matching the disease_pressure additive-term style just above. Below
    // that threshold cortisol contributes nothing extra here: a short-term
    // stress response is adaptive, not harmful, and psychology::update_mental_state
    // already accounts for stress's own direct HP cost separately.
    //
    // KNOWN CALIBRATION GAP (found by empirical_validation.rs's Monte Carlo
    // harness, pre-existing, not introduced by the wound-collapse rewrite
    // above): non-founder 5-15y individuals in practice run chronic cortisol
    // high enough, often enough, that this one term alone contributes
    // roughly 0.02/yr of extra mortality risk on top of everything else --
    // more than the entire age band's ~0.01/yr documented target by itself,
    // and the dominant reason that band's emergent mortality still measured
    // ~3-4x over target even after every wound-collapse-related constant in
    // this module was recalibrated. Direct instrumentation (bypassing the
    // Monte Carlo harness to sample this specific quantity) found juveniles
    // sit with cortisol averaging ~0.1 over this 0.6 threshold for a large
    // fraction of their lives -- i.e. the psychology/hormones stress model
    // itself keeps young non-founders chronically stressed far more than
    // adults, not this term's own coefficient being wrong in isolation.
    // Fixing that is a psychology.rs/hormones.rs stress-model investigation,
    // out of scope for the wound-collapse work this module's other comments
    // document -- left here as an accurate pointer for whoever picks it up
    // next, rather than a blind coefficient tweak that would just mask a
    // stress-model bug behind an unrelated mortality-formula discount.
    if individual.hormones.cortisol > 0.6 {
        base_risk += (individual.hormones.cortisol - 0.6) * 0.0006;
    }

    // >= (not strictly >) 0.25: a strict "> 0.25" gate would never fire for
    // the single most common inbreeding scenario a small founder population
    // hits -- full-sibling or parent-offspring mating, which produces
    // exactly F = 0.25 (see genome.rs::compute_inbreeding_coefficient).
    if individual.inbreeding_coeff.unwrap_or(0.0) >= 0.25 {
        base_risk *= 1.5;
    }

    if is_founder {
        base_risk *= 0.5;
    }

    base_risk.clamp(0.0, 0.99)
}

pub fn roll_death(individual: &Individual, current_day: i32, environment: Option<&Value>) -> Option<DeathCause> {
    if rand::random::<f64>() < compute_daily_death_risk(individual, current_day, environment) {
        Some(determine_cause(individual, current_day, environment))
    } else {
        None
    }
}

fn has_lethal_infection(individual: &Individual) -> bool {
    individual
        .extra
        .get("infections")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter().any(|inf| {
                inf.get("pathogen_id")
                    .and_then(Value::as_str)
                    .and_then(|pid| PATHOGEN_TYPES.iter().find(|(id, ..)| *id == pid))
                    .map(|(_, _, base_mortality, ..)| *base_mortality >= 0.05)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn determine_cause(individual: &Individual, current_day: i32, _environment: Option<&Value>) -> DeathCause {
    let age = get_age(individual, current_day);
    let health = &individual.health;
    let phenotype = &individual.phenotype;
    let is_founder = individual.is_founder;

    if individual._in_water() {
        return DeathCause::Drowning;
    }
    if health.hydration < 0.1 {
        return DeathCause::Dehydration;
    }
    if health.calories < 0.05 {
        return DeathCause::Starvation;
    }
    if has_lethal_infection(individual) {
        return DeathCause::Infection;
    }
    if age >= phenotype.max_lifespan - 5.0 {
        return DeathCause::OldAge;
    }

    // Predator, Injury, WildlifeEncounter, and Exposure used to be resolved
    // right here -- a narrative label picked by an internal probability
    // roll for a death `roll_death`'s own daily risk had *already* decided
    // was happening, with no requirement that the individual had actually
    // sustained any physical harm first. That's the "a probability roll,
    // not the natural consequence of a process" critique this rewrite
    // answers: those four causes are now resolved exclusively by
    // `wounds::wound_collapse_cause` (biology/wounds.rs), which only ever
    // fires when an individual's own accumulated, genetics-modulated,
    // healing-eligible open wounds have driven their hp to 0 -- a real,
    // deterministic physiological outcome, not a coin flip layered under
    // an unrelated daily death roll. `compute_daily_death_risk`'s own
    // base_risk below is reduced (see `NON_WOUND_CAUSE_SHARE`) to hand the
    // portion of overall mortality these four causes used to represent
    // over to that mechanism instead of simply deleting it, so total
    // mortality (validated in `empirical_validation.rs`) stays calibrated.
    let genetic_resistance = (phenotype.health_resilience + phenotype.immune_strength) / 2.0;
    // Founders never die of genetic disease -- the player designed their genome intentionally.
    let genetic_chance = if is_founder { 0.0 } else { (0.30 - genetic_resistance * 0.30).max(0.0) };

    let birth_comp_chance = if individual.sex == "female" && health.pregnancy.is_some() {
        (0.15 - phenotype.fertility * 0.15).max(0.0)
    } else {
        0.0
    };

    // Young-age mortality that isn't drowning/dehydration/starvation/
    // infection/old-age/wound-collapse and isn't attributable to genetic
    // disease has no remaining roll_death-resolvable category left --
    // GeneticDisease is the residual "not otherwise explained" bucket here,
    // same role it already plays as the adult/elder cascades' own fallback
    // below. A child's genetic_resistance/toughness still meaningfully
    // shape their *overall* survival odds (compute_daily_death_risk's own
    // immune_strength/resilience discounts, plus wounds.rs's
    // health_resilience/immune_strength-modulated healing rate) even though
    // this specific label no longer splits on them directly.
    if age < 15.0 {
        return DeathCause::GeneticDisease;
    }

    if age < 45.0 {
        let r = rand::random::<f64>();
        return if r < birth_comp_chance { DeathCause::BirthComplications } else { DeathCause::GeneticDisease };
    }

    let r = rand::random::<f64>();
    let old_age_cut = 0.20;
    let genetic_cut = old_age_cut + genetic_chance;
    if r < old_age_cut {
        DeathCause::OldAge
    } else if r < genetic_cut {
        DeathCause::GeneticDisease
    } else {
        DeathCause::OldAge
    }
}

/// Resolves what used to be a single flat `Trauma` cause into the specific
/// circumstance the current environment signal actually supports, instead of
/// an unexplained catch-all label:
/// 1. Actively dangerous weather (`weather_cold_risk`/`weather_heat_risk`)
///    -- hypothermia/heatstroke.
/// 2. Otherwise, a chance proportional to this biome's `predator_risk` of a
///    smaller, non-apex animal encounter (bite/sting/goring) -- distinct
///    from the dedicated `Predator` cause above, which requires a biome
///    dangerous enough to host a large carnivore.
/// 3. Otherwise, a residual physical mishap (fall, blunt injury, tool
///    accident) -- kept as narrow as the available signals allow rather
///    than an unexplained bucket.
pub(crate) fn resolve_misadventure(environment: Option<&Value>) -> DeathCause {
    if let Some(env) = environment {
        let cold_risk = env.get("weather_cold_risk").and_then(Value::as_bool).unwrap_or(false);
        let heat_risk = env.get("weather_heat_risk").and_then(Value::as_bool).unwrap_or(false);
        if cold_risk || heat_risk {
            return DeathCause::Exposure;
        }
        let predator_risk = env.get("predator_risk").and_then(Value::as_f64).unwrap_or(0.0);
        if rand::random::<f64>() < predator_risk * 0.4 {
            return DeathCause::WildlifeEncounter;
        }
    }
    DeathCause::Injury
}

trait IndividualExt {
    fn _water_experience(&self) -> f64;
    fn _in_water(&self) -> bool;
}

impl IndividualExt for Individual {
    fn _water_experience(&self) -> f64 {
        self.extra.get("_waterExperience").and_then(|v| v.as_f64()).unwrap_or(0.0)
    }
    fn _in_water(&self) -> bool {
        self.extra.get("_inWater").and_then(|v| v.as_bool()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype};

    fn make_ind(age_years: i32) -> Individual {
        Individual {
            birth_day: -age_years * 365,
            phenotype: Phenotype { max_lifespan: 70.0, immune_strength: 0.5, ..Default::default() },
            health: Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
            inbreeding_coeff: Some(0.0),
            ..Default::default()
        }
    }

    fn env(alive_count: f64) -> Value {
        serde_json::json!({ "alive_count": alive_count })
    }

    // ── baseline age bands ───────────────────────────────────────────────

    #[test]
    fn infant_base_risk_is_small_but_positive() {
        let ind = make_ind(0); // birth_day 0, effectively newborn at day 0
        let risk = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        assert!(risk > 0.0 && risk < 0.001);
    }

    #[test]
    fn risk_increases_with_age_band() {
        let adult = make_ind(25);
        let middle_age = make_ind(55);
        let old = make_ind(72);
        let r_a = compute_daily_death_risk(&adult, 0, Some(&env(100.0)));
        let r_m = compute_daily_death_risk(&middle_age, 0, Some(&env(100.0)));
        let r_o = compute_daily_death_risk(&old, 0, Some(&env(100.0)));
        assert!(r_m > r_a);
        assert!(r_o > r_m);
    }

    #[test]
    fn annual_infant_mortality_is_about_7_7_percent() {
        let annual = 1.0 - (1.0 - 0.00022_f64).powi(365);
        assert!((annual - 0.077).abs() < 0.01);
    }

    #[test]
    fn annual_elder_mortality_is_about_20_percent() {
        let annual = 1.0 - (1.0 - 0.00061_f64).powi(365);
        assert!((annual - 0.20).abs() < 0.05);
    }

    // ── multipliers ─────────────────────────────────────────────────────

    #[test]
    fn dehydration_multiplies_risk_heavily() {
        let normal = make_ind(25);
        let mut dehydrated = make_ind(25);
        dehydrated.health.hydration = 0.05;
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_d = compute_daily_death_risk(&dehydrated, 0, Some(&env(100.0)));
        assert!(r_d > r_n * 5.0);
    }

    #[test]
    fn starvation_raises_risk() {
        let normal = make_ind(25);
        let mut starving = make_ind(25);
        starving.health.calories = 0.05;
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_s = compute_daily_death_risk(&starving, 0, Some(&env(100.0)));
        assert!(r_s > r_n * 3.0);
    }

    #[test]
    fn thriving_adult_has_lower_risk_than_baseline() {
        let mut thriving = make_ind(25);
        thriving.health.hp = 0.9;
        thriving.health.calories = 0.8;
        let mut baseline = make_ind(25);
        baseline.health.hp = 0.5;
        baseline.health.calories = 0.5;
        assert!(compute_daily_death_risk(&thriving, 0, Some(&env(100.0))) < compute_daily_death_risk(&baseline, 0, Some(&env(100.0))));
    }

    #[test]
    fn inbreeding_above_quarter_multiplies_risk_by_1_5() {
        let normal = make_ind(25);
        let mut inbred = make_ind(25);
        inbred.inbreeding_coeff = Some(0.6);
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_i = compute_daily_death_risk(&inbred, 0, Some(&env(100.0)));
        assert!((r_i - r_n * 1.5).abs() < 1e-6);
    }

    #[test]
    fn inbreeding_exactly_at_the_quarter_threshold_gets_the_multiplier() {
        // F = 0.25 is exactly what a full-sibling or parent-child mating
        // produces (see genome.rs::compute_inbreeding_coefficient) -- the
        // dominant real-world scenario a small founder population hits, so
        // this boundary case must be covered, not excluded.
        let mut at_threshold = make_ind(25);
        at_threshold.inbreeding_coeff = Some(0.25);
        let normal = make_ind(25);
        let r_at = compute_daily_death_risk(&at_threshold, 0, Some(&env(100.0)));
        let r_normal = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        assert!((r_at - r_normal * 1.5).abs() < 1e-6);
    }

    #[test]
    fn inbreeding_just_below_the_quarter_threshold_gets_no_multiplier() {
        let mut just_below = make_ind(25);
        just_below.inbreeding_coeff = Some(0.24);
        let normal = make_ind(25);
        let r_below = compute_daily_death_risk(&just_below, 0, Some(&env(100.0)));
        let r_normal = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        assert!((r_below - r_normal).abs() < 1e-8);
    }

    #[test]
    fn risk_is_capped_at_0_99() {
        let mut worst = make_ind(100);
        worst.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        worst.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        worst.extra.insert("_inWater".to_string(), serde_json::json!(true));
        assert!(compute_daily_death_risk(&worst, 0, Some(&env(100.0))) <= 0.99);
    }

    // ── water / drowning ────────────────────────────────────────────────

    #[test]
    fn being_in_water_raises_risk() {
        let dry = make_ind(25);
        let mut wet = make_ind(25);
        wet.extra.insert("_inWater".to_string(), serde_json::json!(true));
        assert!(compute_daily_death_risk(&wet, 0, Some(&env(100.0))) > compute_daily_death_risk(&dry, 0, Some(&env(100.0))));
    }

    #[test]
    fn water_experience_lowers_drowning_risk() {
        let mut no_exp = make_ind(25);
        no_exp.extra.insert("_inWater".to_string(), serde_json::json!(true));
        let mut expert = make_ind(25);
        expert.extra.insert("_inWater".to_string(), serde_json::json!(true));
        expert.extra.insert("_waterExperience".to_string(), serde_json::json!(1.0));
        assert!(compute_daily_death_risk(&expert, 0, Some(&env(100.0))) < compute_daily_death_risk(&no_exp, 0, Some(&env(100.0))));
    }

    // ── extinction guard ────────────────────────────────────────────────

    #[test]
    fn small_population_reduces_individual_risk() {
        let ind = make_ind(25);
        let r_large = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        let r_small = compute_daily_death_risk(&ind, 0, Some(&env(5.0)));
        assert!(r_small < r_large);
    }

    #[test]
    fn population_of_one_gets_the_minimum_0_25x_guard() {
        let ind = make_ind(25);
        let r_single = compute_daily_death_risk(&ind, 0, Some(&env(1.0)));
        let r_full = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        assert!((r_single - r_full * 0.25).abs() < 1e-5);
    }

    // ── rollDeath causality ─────────────────────────────────────────────

    #[test]
    fn a_healthy_adult_rarely_dies_in_100_trials() {
        let healthy = make_ind(25);
        let deaths = (0..100).filter(|_| roll_death(&healthy, 0, Some(&env(100.0))).is_some()).count();
        assert!(deaths < 5);
    }

    #[test]
    fn death_returns_a_specific_cause_when_it_happens() {
        let mut ind = make_ind(100);
        ind.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        ind.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        let mut saw_death = false;
        for _ in 0..200 {
            if roll_death(&ind, 0, Some(&env(100.0))).is_some() {
                saw_death = true;
                break;
            }
        }
        assert!(saw_death);
    }

    #[test]
    fn in_water_death_is_always_attributed_to_drowning() {
        let mut ind = make_ind(25);
        ind.extra.insert("_inWater".to_string(), serde_json::json!(true));
        ind.extra.insert("_waterExperience".to_string(), serde_json::json!(0.0));
        ind.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        let mut saw_drowning = false;
        for _ in 0..5000 {
            if roll_death(&ind, 0, Some(&env(100.0))) == Some(DeathCause::Drowning) {
                saw_drowning = true;
                break;
            }
        }
        assert!(saw_drowning);
    }

    #[test]
    fn severe_dehydration_is_attributed_to_dehydration() {
        let mut ind = make_ind(90);
        ind.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        ind.health = Health { hp: 0.1, calories: 0.8, hydration: 0.05, ..Default::default() };
        let mut saw_dehydration = false;
        for _ in 0..200 {
            if roll_death(&ind, 0, Some(&env(100.0))) == Some(DeathCause::Dehydration) {
                saw_dehydration = true;
                break;
            }
        }
        assert!(saw_dehydration);
    }

    // ── young-age cause resolution: misadventure moved to wounds.rs ──────
    //
    // determine_cause used to split under-15 deaths between GeneticDisease
    // and a misadventure cause (Exposure/WildlifeEncounter/Injury) using a
    // genetic_resistance/toughness-modulated share -- the tests this
    // replaces (`an_under_five_with_average_genetics_matches_the_original_
    // flat_split` and its siblings) exercised that split. Predator/Injury/
    // WildlifeEncounter/Exposure are no longer resolved by determine_cause
    // at any age (see this function's own doc comment) -- a young
    // individual's genetic_resistance/toughness still meaningfully shape
    // their *overall* survival odds via compute_daily_death_risk's
    // immune_strength/resilience discounts and wounds.rs's own
    // health_resilience/immune_strength-modulated healing rate, just not
    // through this label-picking split anymore. What remains true and
    // worth testing here is the simpler, current behavior: an under-15
    // roll_death death always resolves to GeneticDisease, regardless of
    // genetics -- the genetics-modulated survive-vs-succumb-to-a-wound
    // split now lives in wounds.rs's own test suite instead
    // (`sustained_wounding_faster_than_healing_can_drive_hp_to_zero` and
    // the `wound_collapse_cause` tests).

    fn child_with(age_years: i32, health_resilience: f64, immune_strength: f64, endurance: f64, physical_strength: f64) -> Individual {
        let mut ind = make_ind(age_years);
        ind.phenotype = Phenotype { max_lifespan: 70.0, health_resilience, immune_strength, endurance, physical_strength, ..Default::default() };
        ind.health = Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() };
        ind
    }

    #[test]
    fn an_under_fifteen_roll_death_always_resolves_to_genetic_disease_regardless_of_genetics() {
        for ind in [child_with(2, 0.5, 0.5, 0.5, 0.5), child_with(10, 0.9, 0.9, 0.9, 0.9), child_with(10, 0.0, 0.0, 0.0, 0.0)] {
            for _ in 0..200 {
                assert_eq!(determine_cause(&ind, 0, Some(&env(100.0))), DeathCause::GeneticDisease);
            }
        }
    }

    // ── resolve_misadventure sub-cause resolution ─────────────────────────

    #[test]
    fn cold_weather_risk_always_resolves_to_exposure() {
        let env = serde_json::json!({ "weather_cold_risk": true, "predator_risk": 0.5 });
        for _ in 0..200 {
            assert_eq!(resolve_misadventure(Some(&env)), DeathCause::Exposure);
        }
    }

    #[test]
    fn heat_weather_risk_always_resolves_to_exposure() {
        let env = serde_json::json!({ "weather_heat_risk": true, "predator_risk": 0.5 });
        for _ in 0..200 {
            assert_eq!(resolve_misadventure(Some(&env)), DeathCause::Exposure);
        }
    }

    #[test]
    fn no_weather_or_predator_signal_resolves_to_injury() {
        let env = serde_json::json!({});
        for _ in 0..200 {
            assert_eq!(resolve_misadventure(Some(&env)), DeathCause::Injury);
        }
    }

    #[test]
    fn missing_environment_resolves_to_injury() {
        for _ in 0..200 {
            assert_eq!(resolve_misadventure(None), DeathCause::Injury);
        }
    }

    #[test]
    fn a_high_predator_risk_biome_sometimes_yields_a_wildlife_encounter() {
        let env = serde_json::json!({ "predator_risk": 0.5 });
        let mut saw_wildlife = false;
        for _ in 0..2000 {
            if resolve_misadventure(Some(&env)) == DeathCause::WildlifeEncounter {
                saw_wildlife = true;
                break;
            }
        }
        assert!(saw_wildlife, "a biome with real predator_risk should occasionally attribute misadventure to a wildlife encounter");
    }

    #[test]
    fn a_predator_free_biome_never_yields_a_wildlife_encounter() {
        let env = serde_json::json!({ "predator_risk": 0.0 });
        for _ in 0..500 {
            assert_ne!(resolve_misadventure(Some(&env)), DeathCause::WildlifeEncounter);
        }
    }
}
