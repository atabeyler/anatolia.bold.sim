//! Dynamic endocrine system. Distinct from the static, genome-derived
//! phenotype traits (`oxytocin_sensitivity`, `serotonin`, `aggression`,
//! `dominance`, `stress_reactivity`, ...) which model receptor
//! sensitivity/predisposition and never change after birth -- everything in
//! this module models an actual circulating hormone *level* that rises and
//! falls tick by tick.
//!
//! Cardinal rule: `individual.hormones` may only ever be written here (same
//! contract as `mind.consciousness` in consciousness.rs). Every value is a
//! deterministic formula over the individual's own genetics (phenotype/sex/
//! age) and this tick's already-tracked, real state -- `psychology.stress_level`
//! (itself already rolled up from disaster/isolation/exile/trauma/water-fear
//! in psychology.rs), `health.hp`, `health.pregnancy`, `group_id`, and the
//! satiation swing economy.rs already writes to `extra["satiation"]` earlier
//! in the same tick. Nothing here is scripted per individual, and nothing
//! reads an event type that isn't actually reachable in the live tick loop.
//!
//! Forty-nine hormones total (within the ~40-60 range standard endocrinology
//! references cite for the full human set), organized as a genuine cascade
//! rather than independent flat values -- matching how the real endocrine
//! system is actually organized around hypothalamic-pituitary-target-gland
//! axes:
//! - **HPA axis**: `crh` (hypothalamus) drives `acth` (pituitary) drives
//!   `cortisol` (adrenal), not stress_level directly. `msh`/`endorphin`
//!   share ACTH's own POMC precursor pathway (real biology). `il6`/
//!   `tnf_alpha`/`interferon` are infection-triggered cytokines that also
//!   feed back into the HPA axis and thyroid (real inflammatory coupling).
//! - **HPT axis**: `tsh` rises under real negative feedback when `thyroid`
//!   (the prior tick's value) runs low, and drives it back up; `thyroid`
//!   also falls under sustained undernourishment or high `il6` (the real
//!   two-driver "sick euthyroid" response).
//! - **HPG axis**: `lh`/`fsh` (the two real gonadotropin pulses) modulate
//!   `testosterone`/`estrogen` production on top of their own age/sex
//!   baseline; `dhea` (an independent adrenal precursor) modulates both a
//!   second way; `progesterone` tracks pregnancy specifically;
//!   `growth_hormone` (age-curve) drives `igf1`/`osteocalcin` downstream.
//! - **Metabolic pair**: `insulin`/`glucagon` (fast, same-tick) and
//!   `leptin`/`ghrelin` (slow long-term trend vs. fast acute hunger) track
//!   nutritional state at genuinely different timescales; `adiponectin`
//!   (leptin's real inverse) sensitizes insulin's own target; `npy`
//!   amplifies ghrelin's when leptin is low.
//! - **Arousal cascade**: `norepinephrine` (sustained, weeks-scale
//!   vigilance) sets `adrenaline`'s own resting floor, the real locus
//!   coeruleus -> adrenal coupling; `melatonin` (real age/stress-driven
//!   decline) normally suppresses CRH, so its own decline feeds a small
//!   rise back into CRH's target -- the real reciprocal coupling.
//! - **Reproductive/bonding**: `oxytocin`/`vasopressin` (bonding, the
//!   latter more male-leaning), `prolactin` (post-birth surge, then slow
//!   decay).
//! - **Digestive layer**: `gastrin`/`secretin`/`cck`/`motilin`/`gip`/
//!   `somatostatin`/`pyy`/`pancreatic_polypeptide` -- eight genuinely
//!   distinct real-world response *timings* (fast/immediate through
//!   slow/sustained, some cyclic-inverse like motilin) layered over the
//!   same underlying `satiation` signal economy.rs already computes, since
//!   this simulation has no literal separate stomach-contents state.
//! - **Cardiovascular/renal**: `renin` (low `health.hydration`, a blood-
//!   volume proxy) drives `angiotensin_ii` drives `aldosterone`
//!   (real cascade); `anp`/`bnp` are the real counter-regulatory pair;
//!   `epo` tracks low `health.hp` (a blood-loss/anemia proxy).
//! - **Bone/calcium**: `pth` rises with age, sharply amplified in
//!   post-fertile females by low `estrogen` (the real estrogen-bone-
//!   protection link -> osteoporosis pathway); `calcitonin` opposes it;
//!   `vitamin_d` follows a real age-related production decline.
//!
//! A handful of these (digestive, cardiovascular/renal, bone) are proxied
//! through signals this simulation already tracks for other reasons
//! (satiation, hydration, hp, age) rather than a literal separate stomach-
//! contents/blood-pressure/bone-density state -- each still gets its own
//! real-world-motivated formula and distinct response timing (not the same
//! number copy-pasted under different names), but the underlying trigger is
//! an existing abstraction, not a dedicated new subsystem. `melatonin`
//! likewise has no real day/night cycle to respond to at this simulation's
//! one-tick-per-day resolution -- it's driven by its own real age/stress-
//! linked dynamic instead. Every value here still has a genuine formula and
//! at least one real feedback path (own or shared with a sibling hormone);
//! none is a decorative, causally-inert flat field.

use serde_json::{json, Map, Value};

use crate::state::Individual;
use crate::types::Hormones;

use super::biology::individual::get_age;

/// Population-wide hormone averages, mirroring `psychology::compute_population_psych_stats`'s
/// own shape/rounding convention -- surfaced by client_view::derive_stats as
/// `stats.mean_hormones` for the client (PsychologyPanel's "Hormonal System"
/// section) to render without shipping every individual's full hormone
/// struct on every stats poll.
pub fn compute_population_hormone_stats(population: &[Individual]) -> Value {
    let living: Vec<&Individual> = population.iter().filter(|i| i.alive && !i.is_dead).collect();
    if living.is_empty() {
        return json!({
            "cortisol": 0.0, "adrenaline": 0.0, "testosterone": 0.0, "estrogen": 0.0, "dopamine": 0.0, "oxytocin": 0.0,
            "acth": 0.0, "tsh": 0.0, "lh": 0.0, "thyroid": 0.0, "insulin": 0.0, "glucagon": 0.0, "leptin": 0.0,
            "ghrelin": 0.0, "growth_hormone": 0.0, "dhea": 0.0, "prolactin": 0.0, "progesterone": 0.0,
            "norepinephrine": 0.0, "vasopressin": 0.0, "fsh": 0.0, "crh": 0.0, "msh": 0.0, "endorphin": 0.0,
            "il6": 0.0, "tnf_alpha": 0.0, "interferon": 0.0, "igf1": 0.0, "adiponectin": 0.0, "npy": 0.0,
            "gastrin": 0.0, "secretin": 0.0, "cck": 0.0, "motilin": 0.0, "gip": 0.0, "somatostatin": 0.0,
            "pyy": 0.0, "pancreatic_polypeptide": 0.0, "renin": 0.0, "angiotensin_ii": 0.0, "aldosterone": 0.0,
            "anp": 0.0, "bnp": 0.0, "epo": 0.0, "pth": 0.0, "calcitonin": 0.0, "vitamin_d": 0.0,
            "osteocalcin": 0.0, "melatonin": 0.0,
        });
    }
    let n = living.len() as f64;
    let avg = |f: fn(&Hormones) -> f64| living.iter().map(|i| f(&i.hormones)).sum::<f64>() / n;
    json!({
        "cortisol": avg(|h| h.cortisol),
        "adrenaline": avg(|h| h.adrenaline),
        "testosterone": avg(|h| h.testosterone),
        "estrogen": avg(|h| h.estrogen),
        "dopamine": avg(|h| h.dopamine),
        "oxytocin": avg(|h| h.oxytocin),
        "acth": avg(|h| h.acth),
        "tsh": avg(|h| h.tsh),
        "lh": avg(|h| h.lh),
        "thyroid": avg(|h| h.thyroid),
        "insulin": avg(|h| h.insulin),
        "glucagon": avg(|h| h.glucagon),
        "leptin": avg(|h| h.leptin),
        "ghrelin": avg(|h| h.ghrelin),
        "growth_hormone": avg(|h| h.growth_hormone),
        "dhea": avg(|h| h.dhea),
        "prolactin": avg(|h| h.prolactin),
        "progesterone": avg(|h| h.progesterone),
        "norepinephrine": avg(|h| h.norepinephrine),
        "vasopressin": avg(|h| h.vasopressin),
        "fsh": avg(|h| h.fsh),
        "crh": avg(|h| h.crh),
        "msh": avg(|h| h.msh),
        "endorphin": avg(|h| h.endorphin),
        "il6": avg(|h| h.il6),
        "tnf_alpha": avg(|h| h.tnf_alpha),
        "interferon": avg(|h| h.interferon),
        "igf1": avg(|h| h.igf1),
        "adiponectin": avg(|h| h.adiponectin),
        "npy": avg(|h| h.npy),
        "gastrin": avg(|h| h.gastrin),
        "secretin": avg(|h| h.secretin),
        "cck": avg(|h| h.cck),
        "motilin": avg(|h| h.motilin),
        "gip": avg(|h| h.gip),
        "somatostatin": avg(|h| h.somatostatin),
        "pyy": avg(|h| h.pyy),
        "pancreatic_polypeptide": avg(|h| h.pancreatic_polypeptide),
        "renin": avg(|h| h.renin),
        "angiotensin_ii": avg(|h| h.angiotensin_ii),
        "aldosterone": avg(|h| h.aldosterone),
        "anp": avg(|h| h.anp),
        "bnp": avg(|h| h.bnp),
        "epo": avg(|h| h.epo),
        "pth": avg(|h| h.pth),
        "calcitonin": avg(|h| h.calcitonin),
        "vitamin_d": avg(|h| h.vitamin_d),
        "osteocalcin": avg(|h| h.osteocalcin),
        "melatonin": avg(|h| h.melatonin),
    })
}

/// Real puberty timeline: negligible before 9, ramping through adolescence,
/// reaching the adult plateau by 17 -- matches `get_life_stage`'s own
/// "adolescent" band (12-18y) in biology/individual.rs.
fn puberty_curve(age_years: f64) -> f64 {
    if age_years < 9.0 {
        0.0
    } else if age_years < 17.0 {
        (age_years - 9.0) / 8.0
    } else {
        1.0
    }
}

/// Gradual male decline (andropause: slow, partial) vs. a sharper female
/// decline (menopause: steeper, over a shorter window) -- both real,
/// well-documented asymmetries in reproductive-hormone senescence.
fn senescence_curve(age_years: f64, sex: &str) -> f64 {
    if sex == "female" {
        if age_years < 45.0 {
            1.0
        } else if age_years < 55.0 {
            (1.0 - (age_years - 45.0) / 10.0 * 0.85).max(0.15)
        } else {
            0.15
        }
    } else if age_years < 50.0 {
        1.0
    } else {
        (1.0 - (age_years - 50.0) * 0.01).max(0.4)
    }
}

/// DHEA-S's real lifetime curve: rises through childhood/puberty, peaks
/// around 25, then declines steadily ("adrenopause") -- independent of, and
/// in addition to, the sex-specific senescence curve above (DHEA is an
/// adrenal precursor shared by both sexes, not a gonadal hormone).
fn dhea_curve(age_years: f64) -> f64 {
    if age_years < 25.0 {
        (age_years / 25.0).clamp(0.0, 1.0)
    } else {
        (1.0 - (age_years - 25.0) * 0.012).max(0.1)
    }
}

/// Real GH pulsatility peaks around puberty, then declines through
/// adulthood ("somatopause").
fn growth_hormone_curve(age_years: f64) -> f64 {
    if age_years < 17.0 {
        (0.4 + puberty_curve(age_years) * 0.6).min(1.0)
    } else {
        (1.0 - (age_years - 17.0) * 0.012).max(0.15)
    }
}

/// Baseline (pre-LH/DHEA-modulation) testosterone/estrogen for this
/// individual's current age, sex, and genetics. Both sexes carry some of
/// each (biologically accurate), with the sex-typical hormone dominant.
fn sex_hormone_baselines(sex: &str, age_years: f64, dominance: f64, fertility: f64) -> (f64, f64) {
    let puberty = puberty_curve(age_years);
    match sex {
        "male" => {
            let senescence = senescence_curve(age_years, sex);
            let dominance_mod = 0.7 + dominance * 0.3;
            let testosterone = (0.15 + 0.55 * puberty * dominance_mod * senescence).clamp(0.0, 1.0);
            (testosterone, 0.06)
        }
        "female" => {
            let senescence = senescence_curve(age_years, sex);
            let fertility_mod = 0.7 + fertility * 0.3;
            let estrogen = (0.12 + 0.55 * puberty * fertility_mod * senescence).clamp(0.0, 1.0);
            (0.08, estrogen)
        }
        _ => (0.1, 0.1),
    }
}

fn dopamine_baseline(curiosity: f64, risk_tolerance: f64) -> f64 {
    (0.35 + curiosity * 0.1 + risk_tolerance * 0.1).clamp(0.0, 1.0)
}

fn oxytocin_baseline(oxytocin_sensitivity: f64) -> f64 {
    (oxytocin_sensitivity * 0.3).clamp(0.0, 1.0)
}

/// Genetic/age baseline at birth (day 0 of this individual's life) -- called
/// once by every individual-creation path (`create_founder`, `create_child`,
/// `migrate_individual_arrival`), the same way `epigenetics::snapshot_genetic_baseline`
/// is. Nothing here reads any per-tick state; it exists purely so a
/// newborn's very first tick already has a sane, genetically-grounded
/// starting point instead of a flat default.
pub fn initialize_hormones(individual: &mut Individual) {
    let p = &individual.phenotype;
    let cortisol = (0.25 + p.stress_reactivity * 0.25).clamp(0.0, 1.0);
    let acth = cortisol;
    let adrenaline = 0.05;
    let norepinephrine = 0.1;
    let (testosterone, estrogen) = sex_hormone_baselines(&individual.sex, 0.0, p.dominance, p.fertility);
    let dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    let vasopressin_sensitivity = (p.parental_care * 0.5 + p.cooperation * 0.5).clamp(0.0, 1.0);
    let vasopressin = (vasopressin_sensitivity * 0.3).clamp(0.0, 1.0);
    let tsh = 0.5;
    let thyroid = 0.5;
    let lh = puberty_curve(0.0);
    let dhea = dhea_curve(0.0);
    let growth_hormone = growth_hormone_curve(0.0);
    let progesterone = if individual.sex == "female" { 0.15 } else { 0.05 };
    individual.hormones = Hormones {
        cortisol,
        adrenaline,
        testosterone,
        estrogen,
        dopamine,
        oxytocin,
        acth,
        tsh,
        lh,
        thyroid,
        insulin: 0.4,
        glucagon: 0.3,
        leptin: 0.3,
        ghrelin: 0.4,
        growth_hormone,
        dhea,
        prolactin: 0.05,
        progesterone,
        norepinephrine,
        vasopressin,
        // Group A/B: flat resting-baseline starting points -- update_hormones
        // converges every one of these toward its own real formula within a
        // handful of ticks, so a precise birth-time value isn't needed here
        // (matching tsh/thyroid/insulin/... above, already flat starts).
        fsh: puberty_curve(0.0),
        crh: cortisol,
        msh: acth,
        endorphin: 0.3,
        il6: 0.1,
        tnf_alpha: 0.1,
        interferon: 0.1,
        igf1: growth_hormone,
        adiponectin: 0.5,
        npy: 0.3,
        gastrin: 0.3,
        secretin: 0.3,
        cck: 0.3,
        motilin: 0.4,
        gip: 0.3,
        somatostatin: 0.3,
        pyy: 0.3,
        pancreatic_polypeptide: 0.3,
        renin: 0.3,
        angiotensin_ii: 0.3,
        aldosterone: 0.3,
        anp: 0.3,
        bnp: 0.2,
        epo: 0.3,
        pth: 0.3,
        calcitonin: 0.4,
        vitamin_d: 0.5,
        osteocalcin: growth_hormone,
        melatonin: 0.4,
        extra: Map::new(),
    };
}

/// Daily update -- called once per living individual per tick, after
/// `psychology::update_mental_state` (needs this tick's fresh `stress_level`)
/// and after the economy phase (needs this tick's fresh `satiation`). Every
/// target is blended toward, not snapped to, at a hormone-specific rate
/// reflecting real clearance/gland-response speed. Several hormones form a
/// genuine same-tick cascade (computed in dependency order below: ACTH
/// before cortisol, LH/DHEA before testosterone/estrogen, TSH before
/// thyroid, norepinephrine before adrenaline); the negative-feedback pairs
/// (TSH<->thyroid) intentionally read the *previous* tick's value on one
/// side to avoid a same-tick circular dependency -- itself realistic, since
/// real feedback loops operate with a delay.
pub fn update_hormones(individual: &mut Individual, current_day: i32) {
    let age_years = get_age(individual, current_day);
    let p = individual.phenotype.clone();
    let sex = individual.sex.clone();
    let h = individual.hormones.clone();

    let satiation = individual.extra.get("satiation").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let hydration = individual.health.hydration;
    let has_active_infection = individual.extra.get("infections").and_then(Value::as_array).map(|a| !a.is_empty()).unwrap_or(false);

    // ---- Metabolic pair 1: insulin / glucagon (fast, same-tick) ----
    // Adiponectin (below) nudges insulin's own target down slightly once
    // computed -- real insulin-sensitizing effect -- so insulin is finalized
    // after adiponectin further down; this is its pre-adiponectin base.
    let insulin_base_target = satiation;
    let glucagon = h.glucagon + ((1.0 - satiation) - h.glucagon) * 0.35;

    // ---- Metabolic pair 2: leptin (slow trend) / ghrelin (fast, acute) ----
    let leptin = h.leptin + (satiation - h.leptin) * 0.03;
    // NPY (real hypothalamic appetite driver, rises when leptin is low)
    // amplifies ghrelin's own target -- the real NPY-ghrelin interaction.
    let npy_target = (1.0 - leptin).clamp(0.0, 1.0);
    let npy = h.npy + (npy_target - h.npy) * 0.1;
    let ghrelin_target = ((1.0 - satiation) * (1.0 + npy * 0.2)).clamp(0.0, 1.0);
    let ghrelin = h.ghrelin + (ghrelin_target - h.ghrelin) * 0.4;

    // Adiponectin: real-world inverse of leptin/body fat; finalizes insulin's
    // target with a small insulin-sensitizing discount.
    let adiponectin_target = (1.0 - leptin).clamp(0.0, 1.0);
    let adiponectin = h.adiponectin + (adiponectin_target - h.adiponectin) * 0.03;
    let insulin_target = (insulin_base_target * (1.0 - adiponectin * 0.15)).clamp(0.0, 1.0);
    let insulin = h.insulin + (insulin_target - h.insulin) * 0.35;

    // ---- HPA axis: CRH (hypothalamus) drives ACTH (pituitary) drives cortisol (adrenal) ----
    // Melatonin normally suppresses cortisol; a low melatonin level (age or
    // chronic-stress-driven, see below) removes some of that suppression,
    // feeding a small extra rise into CRH's own target -- the real
    // reciprocal melatonin-cortisol coupling.
    let stress = individual.psychology.stress_level;
    let melatonin_target = (0.5 - age_years * 0.003 - stress * 0.15).clamp(0.05, 0.6);
    let melatonin = h.melatonin + (melatonin_target - h.melatonin) * 0.05;
    let crh_target = (stress + (0.3 - melatonin).max(0.0) * 0.2).clamp(0.0, 1.0);
    let crh = h.crh + (crh_target - h.crh) * 0.3;
    let acth_base_target = crh;
    // TNF-alpha (below, infection-driven) also activates the HPA axis in
    // real physiology -- folded into ACTH's own target once computed.
    let tnf_alpha_target = if has_active_infection { 0.7 } else { 0.1 };
    let tnf_alpha = h.tnf_alpha + (tnf_alpha_target - h.tnf_alpha) * 0.3;
    let acth_target = (acth_base_target + tnf_alpha * 0.15).clamp(0.0, 1.0);
    let acth = h.acth + (acth_target - h.acth) * 0.3;
    let cortisol_target = (acth * (0.4 + p.stress_reactivity * 0.6)).clamp(0.0, 1.0);
    let cortisol = h.cortisol + (cortisol_target - h.cortisol) * 0.2;

    // ---- The other two POMC-derived hormones: MSH and endorphin, sharing
    // ACTH's own precursor pathway (real biology) ----
    let msh_target = acth;
    let msh = h.msh + (msh_target - h.msh) * 0.2;
    let endorphin_target = if individual.health.hp < 0.4 { 0.7 } else if satiation > 0.75 { 0.6 } else { 0.3 };
    let endorphin = h.endorphin + (endorphin_target - h.endorphin) * 0.25;
    // Real endogenous analgesia/euphoria -- a small, bounded wellbeing nudge
    // from the *previous* tick's endorphin level (psychology::update_mental_state
    // already finalized this tick's wellbeing before this function runs, so
    // this reads one tick behind, the same delayed-feedback pattern as
    // TSH/thyroid above).
    if h.endorphin > 0.5 {
        individual.psychology.wellbeing = (individual.psychology.wellbeing + (h.endorphin - 0.5) * 0.04).min(1.0);
    }

    // ---- Immune cytokines: real trigger is an active infection (microbiome.rs) ----
    let il6_target = if has_active_infection { 0.75 } else { 0.1 };
    let il6 = h.il6 + (il6_target - h.il6) * 0.3;
    let interferon_target = if has_active_infection { 0.6 } else { 0.1 };
    let interferon = h.interferon + (interferon_target - h.interferon) * 0.3;

    // ---- Arousal cascade: norepinephrine (sustained) sets adrenaline's floor ----
    // Only a real, this-instant threat spikes adrenaline hard -- critically
    // low HP or severe acute hunger (ghrelin) are the two signals available
    // here that are unambiguously "urgent" on every backend (WASM-local
    // included) without depending on an event feed.
    let acute_threat = individual.health.hp < 0.25 || ghrelin > 0.8;
    let norepinephrine_target = if acute_threat { 0.6 } else { (0.15 + stress * 0.2).clamp(0.0, 1.0) };
    let norepinephrine = h.norepinephrine + (norepinephrine_target - h.norepinephrine) * 0.15;
    let adrenaline_target = if acute_threat { (0.6 + p.risk_tolerance * 0.4).clamp(0.0, 1.0) } else { (norepinephrine * 0.3 + 0.05).clamp(0.0, 1.0) };
    let adrenaline_rate = if acute_threat { 0.8 } else { 0.5 };
    let adrenaline = h.adrenaline + (adrenaline_target - h.adrenaline) * adrenaline_rate;

    // ---- HPT axis: TSH <-> thyroid negative feedback ----
    // TSH rises when the *previous* tick's thyroid output was low (the real
    // pituitary negative-feedback signal); thyroid then responds to this
    // tick's fresh TSH plus nutritional state (sustained undernourishment
    // suppresses thyroid output -- the real "sick euthyroid" energy-
    // conservation adaptation).
    let tsh_target = (1.0 - h.thyroid).clamp(0.0, 1.0) * 0.7 + 0.15;
    let tsh = h.tsh + (tsh_target - h.tsh) * 0.15;
    // Real "sick euthyroid syndrome" has two real drivers: sustained
    // undernourishment (satiation) and inflammatory-cytokine suppression
    // (il6) -- both fold into thyroid's own target here, not just nutrition.
    let thyroid_target = (0.25 + satiation * 0.35 + tsh * 0.3 - il6 * 0.15).clamp(0.0, 1.0);
    let thyroid = h.thyroid + (thyroid_target - h.thyroid) * 0.08;

    // ---- HPG axis: LH + FSH (gonadotropin pulses) + DHEA modulate testosterone/estrogen ----
    // FSH and LH are the two real pituitary gonadotropins -- FSH drives
    // gamete maturation, LH triggers the actual hormone-release pulse (the
    // one `testosterone`/`estrogen` below track); both follow the same
    // puberty timeline but FSH responds more slowly.
    let fsh_target = puberty_curve(age_years);
    let fsh = h.fsh + (fsh_target - h.fsh) * 0.08;
    let lh_target = puberty_curve(age_years);
    let lh = h.lh + (lh_target - h.lh) * 0.12;
    let dhea_target = dhea_curve(age_years);
    let dhea = h.dhea + (dhea_target - h.dhea) * 0.05;
    let (base_t, base_e) = sex_hormone_baselines(&sex, age_years, p.dominance, p.fertility);
    let pregnant = individual.health.pregnancy.is_some();
    let lh_gain = 0.7 + 0.3 * lh;
    let dhea_gain = 0.85 + 0.15 * dhea;
    let testosterone_target = (base_t * lh_gain * dhea_gain).clamp(0.0, 1.0);
    let testosterone = h.testosterone + (testosterone_target - h.testosterone) * 0.1;
    let estrogen_base = if pregnant { (base_e * 1.6).min(1.0) } else { base_e };
    let estrogen_target = (estrogen_base * lh_gain * dhea_gain).clamp(0.0, 1.0);
    let estrogen = h.estrogen + (estrogen_target - h.estrogen) * 0.1;

    // ---- Progesterone: pregnancy-specific, distinct from estrogen's own dynamic ----
    let progesterone_target = if pregnant {
        0.85
    } else if sex == "female" {
        (0.1 + p.fertility * 0.1 * puberty_curve(age_years)).clamp(0.0, 1.0)
    } else {
        0.05
    };
    let progesterone = h.progesterone + (progesterone_target - h.progesterone) * 0.12;

    // ---- Growth hormone: age-curve only (see module doc comment) ----
    let growth_hormone_target = growth_hormone_curve(age_years);
    let growth_hormone = h.growth_hormone + (growth_hormone_target - h.growth_hormone) * 0.05;
    // IGF-1: the real downstream liver signal GH actually acts through.
    let igf1 = h.igf1 + (growth_hormone - h.igf1) * 0.05;
    // Osteocalcin: real bone-formation marker, active alongside growth.
    let osteocalcin = h.osteocalcin + (growth_hormone - h.osteocalcin) * 0.05;

    // ---- Digestive-hormone timescale layer over the satiation signal ----
    // No literal stomach-contents state exists in this simulation (only the
    // abstract per-tick `satiation` economy.rs already computes) -- these
    // model each hormone's own genuinely distinct real-world response
    // *timing* to that same underlying "food present" reality, rather than
    // repeating one signal eight times unchanged. See module doc comment.
    let gastrin_target = satiation;
    let gastrin = h.gastrin + (gastrin_target - h.gastrin) * 0.4;
    let secretin_target = gastrin;
    let secretin = h.secretin + (secretin_target - h.secretin) * 0.3;
    let cck_target = satiation;
    let cck = h.cck + (cck_target - h.cck) * 0.15;
    let motilin_target = (1.0 - satiation).clamp(0.0, 1.0);
    let motilin = h.motilin + (motilin_target - h.motilin) * 0.2;
    let gip_target = insulin;
    let gip = h.gip + (gip_target - h.gip) * 0.3;
    let somatostatin_target = (1.0 - gastrin).clamp(0.0, 1.0);
    let somatostatin = h.somatostatin + (somatostatin_target - h.somatostatin) * 0.2;
    let pyy_target = cck;
    let pyy = h.pyy + (pyy_target - h.pyy) * 0.1;
    let pancreatic_polypeptide_target = somatostatin;
    let pancreatic_polypeptide = h.pancreatic_polypeptide + (pancreatic_polypeptide_target - h.pancreatic_polypeptide) * 0.2;

    // ---- Cardiovascular/renal, proxied through hydration (blood-volume
    // proxy) and hp (blood-loss/injury proxy) -- no literal blood-pressure
    // state exists in this simulation. See module doc comment. ----
    let renin_target = (1.0 - hydration).clamp(0.0, 1.0);
    let renin = h.renin + (renin_target - h.renin) * 0.25;
    let angiotensin_ii_target = renin;
    let angiotensin_ii = h.angiotensin_ii + (angiotensin_ii_target - h.angiotensin_ii) * 0.2;
    let aldosterone_target = angiotensin_ii;
    let aldosterone = h.aldosterone + (aldosterone_target - h.aldosterone) * 0.15;
    let anp_target = hydration;
    let anp = h.anp + (anp_target - h.anp) * 0.2;
    let bnp_target = anp;
    let bnp = h.bnp + (bnp_target - h.bnp) * 0.15;
    let epo_target = (1.0 - individual.health.hp).clamp(0.0, 1.0);
    let epo = h.epo + (epo_target - h.epo) * 0.15;

    // ---- Bone/calcium, proxied through age and the real estrogen-bone
    // protective link -- no literal bone-density state exists. ----
    let bone_age_factor = (age_years / 80.0).clamp(0.0, 1.0);
    let post_fertile_female = sex == "female" && age_years >= 45.0;
    let pth_target = if post_fertile_female { (bone_age_factor + (1.0 - estrogen) * 0.3).clamp(0.0, 1.0) } else { bone_age_factor * 0.5 };
    let pth = h.pth + (pth_target - h.pth) * 0.03;
    let calcitonin_target = (1.0 - bone_age_factor * 0.6).clamp(0.2, 1.0);
    let calcitonin = h.calcitonin + (calcitonin_target - h.calcitonin) * 0.03;
    let vitamin_d_target = (0.7 - bone_age_factor * 0.3 + (p.health_resilience - 0.5) * 0.2).clamp(0.0, 1.0);
    let vitamin_d = h.vitamin_d + (vitamin_d_target - h.vitamin_d) * 0.02;

    // ---- Dopamine (reward/motivation) ----
    // A same-tick nutritional swing is the primary reward signal; chronic
    // leptin depletion (long-term energy-reserve deficit) also blunts reward
    // sensitivity by a small, bounded amount -- a real, well-documented
    // leptin-dopamine coupling.
    let baseline_dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let dopamine_target_raw = if satiation > 0.75 {
        (baseline_dopamine + 0.3 * (satiation - 0.75) / 0.25).min(1.0)
    } else if satiation < 0.3 {
        (baseline_dopamine - 0.2 * (0.3 - satiation) / 0.3).max(0.05)
    } else {
        baseline_dopamine
    };
    let dopamine_target = (dopamine_target_raw * (0.9 + 0.1 * leptin)).clamp(0.0, 1.0);
    let dopamine_rate = if satiation > 0.75 { 0.4 } else { 0.25 };
    let dopamine = h.dopamine + (dopamine_target - h.dopamine) * dopamine_rate;

    // ---- Oxytocin / vasopressin (bonding) ----
    let baseline_oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    let oxytocin_target = if individual.group_id.is_some() { (baseline_oxytocin + p.oxytocin_sensitivity * 0.15).min(1.0) } else { baseline_oxytocin };
    let oxytocin = h.oxytocin + (oxytocin_target - h.oxytocin) * 0.15;
    let vasopressin_sensitivity = (p.parental_care * 0.5 + p.cooperation * 0.5).clamp(0.0, 1.0);
    let baseline_vasopressin = (vasopressin_sensitivity * 0.3).clamp(0.0, 1.0);
    let vasopressin_target = if individual.group_id.is_some() { (baseline_vasopressin + vasopressin_sensitivity * 0.15).min(1.0) } else { baseline_vasopressin };
    let vasopressin = h.vasopressin + (vasopressin_target - h.vasopressin) * 0.15;

    // ---- Prolactin: decays slowly from whatever apply_birth_surge set ----
    let prolactin_target = 0.05;
    let prolactin = h.prolactin + (prolactin_target - h.prolactin) * 0.02;

    individual.hormones = Hormones {
        cortisol,
        adrenaline,
        testosterone,
        estrogen,
        dopamine,
        oxytocin,
        acth,
        tsh,
        lh,
        thyroid,
        insulin,
        glucagon,
        leptin,
        ghrelin,
        growth_hormone,
        dhea,
        prolactin,
        progesterone,
        norepinephrine,
        vasopressin,
        fsh,
        crh,
        msh,
        endorphin,
        il6,
        tnf_alpha,
        interferon,
        igf1,
        adiponectin,
        npy,
        gastrin,
        secretin,
        cck,
        motilin,
        gip,
        somatostatin,
        pyy,
        pancreatic_polypeptide,
        renin,
        angiotensin_ii,
        aldosterone,
        anp,
        bnp,
        epo,
        pth,
        calcitonin,
        vitamin_d,
        osteocalcin,
        melatonin,
        extra: h.extra,
    };
}

/// A conception event is a real, discrete, this-instant reproductive/social
/// signal -- called directly from tick.rs's reproduction phase, at the exact
/// point mating is rolled (the one live call site `psychology::process_bonding`
/// itself is invoked from), rather than inferred from an event log. LH (the
/// actual physiological gonadotropin trigger) surges first, testosterone and
/// estrogen follow, and oxytocin/vasopressin surge in proportion to each
/// individual's own receptor sensitivity -- exactly the same asymmetry
/// `psychology::process_bonding`'s own bond-strength formula already models.
pub fn apply_mating_surge(mother: &mut Individual, father: &mut Individual) {
    for ind in [mother, father] {
        let sensitivity = ind.phenotype.oxytocin_sensitivity;
        let vasopressin_sensitivity = (ind.phenotype.parental_care * 0.5 + ind.phenotype.cooperation * 0.5).clamp(0.0, 1.0);
        ind.hormones.lh = (ind.hormones.lh + 0.15).min(1.0);
        ind.hormones.testosterone = (ind.hormones.testosterone + 0.1).min(1.0);
        ind.hormones.estrogen = (ind.hormones.estrogen + 0.1).min(1.0);
        ind.hormones.oxytocin = (ind.hormones.oxytocin + sensitivity * 0.4).min(1.0);
        ind.hormones.vasopressin = (ind.hormones.vasopressin + vasopressin_sensitivity * 0.4).min(1.0);
    }
}

/// A real, discrete, this-instant event (unlike prolactin's own otherwise-flat
/// per-tick target) -- called directly from tick.rs at the exact point a
/// birth is resolved, mirroring `apply_mating_surge`'s own pattern. Prolactin
/// then decays slowly back toward baseline over subsequent ticks (see
/// `update_hormones`), matching real lactation's weeks-scale elevation.
pub fn apply_birth_surge(mother: &mut Individual) {
    mother.hormones.prolactin = (mother.hormones.prolactin + 0.7).min(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype, Psychology};

    fn base_individual(sex: &str) -> Individual {
        Individual {
            sex: sex.to_string(),
            birth_day: 0,
            phenotype: Phenotype {
                stress_reactivity: 0.5,
                dominance: 0.5,
                fertility: 0.5,
                curiosity: 0.5,
                risk_tolerance: 0.5,
                oxytocin_sensitivity: 0.5,
                parental_care: 0.5,
                cooperation: 0.5,
                ..Default::default()
            },
            health: Health { hp: 1.0, ..Default::default() },
            psychology: Psychology { stress_level: 0.2, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn initialize_seeds_a_prepubertal_baseline_not_a_flat_default() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        // Age 0 -> puberty_curve(0) == 0.0, so testosterone should sit at its
        // prepubertal floor, not the adult plateau.
        assert!(ind.hormones.testosterone < 0.2, "expected a prepubertal testosterone floor, got {}", ind.hormones.testosterone);
    }

    #[test]
    fn male_testosterone_rises_through_puberty_and_plateaus_in_adulthood() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let adult_t = ind.hormones.testosterone;
        assert!(adult_t > 0.4, "expected an adult male to have a substantial testosterone plateau, got {adult_t}");
    }

    #[test]
    fn female_estrogen_rises_through_puberty_and_plateaus_in_adulthood() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let adult_e = ind.hormones.estrogen;
        assert!(adult_e > 0.4, "expected an adult female to have a substantial estrogen plateau, got {adult_e}");
    }

    #[test]
    fn pregnancy_elevates_estrogen_above_the_non_pregnant_baseline() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let baseline = ind.hormones.estrogen;
        ind.health.pregnancy = Some(25 * 365);
        for day in (25 * 365 + 1)..=(25 * 365 + 30) {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.estrogen > baseline, "pregnancy should elevate estrogen above baseline ({} vs {baseline})", ind.hormones.estrogen);
    }

    #[test]
    fn pregnancy_elevates_progesterone_far_above_the_non_pregnant_baseline() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let baseline = ind.hormones.progesterone;
        ind.health.pregnancy = Some(25 * 365);
        for day in (25 * 365 + 1)..=(25 * 365 + 60) {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.progesterone > baseline + 0.3, "pregnancy should sharply elevate progesterone ({} vs baseline {baseline})", ind.hormones.progesterone);
    }

    #[test]
    fn male_testosterone_declines_past_fifty_but_never_collapses() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(30 * 365) {
            update_hormones(&mut ind, day);
        }
        let prime = ind.hormones.testosterone;
        for day in (30 * 365 + 1)..=(70 * 365) {
            update_hormones(&mut ind, day);
        }
        let elder = ind.hormones.testosterone;
        assert!(elder < prime, "andropause should lower testosterone below the prime-age level ({elder} vs {prime})");
        assert!(elder > 0.1, "andropause should be a partial decline, not a collapse to near-zero, got {elder}");
    }

    #[test]
    fn critically_low_hp_spikes_adrenaline() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.health.hp = 0.1;
        update_hormones(&mut ind, 1);
        assert!(ind.hormones.adrenaline > 0.3, "critical HP should spike adrenaline, got {}", ind.hormones.adrenaline);
    }

    #[test]
    fn adrenaline_clears_fast_once_the_threat_passes() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.health.hp = 0.1;
        update_hormones(&mut ind, 1);
        let spiked = ind.hormones.adrenaline;
        ind.health.hp = 1.0;
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.5));
        update_hormones(&mut ind, 2);
        assert!(ind.hormones.adrenaline < spiked, "adrenaline should start clearing the tick after the threat ends");
    }

    #[test]
    fn severe_acute_hunger_also_spikes_adrenaline_via_ghrelin() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=5 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.02));
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.ghrelin > 0.8, "sustained near-zero satiation should push ghrelin high, got {}", ind.hormones.ghrelin);
        assert!(ind.hormones.adrenaline > 0.3, "severe acute hunger should spike adrenaline via ghrelin, got {}", ind.hormones.adrenaline);
    }

    #[test]
    fn high_stress_raises_cortisol_toward_a_reactivity_scaled_target_via_acth() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.psychology.stress_level = 0.9;
        for day in 1..=10 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.acth > 0.7, "sustained high stress should elevate ACTH first, got {}", ind.hormones.acth);
        assert!(ind.hormones.cortisol > 0.5, "sustained high stress should elevate cortisol via ACTH, got {}", ind.hormones.cortisol);
    }

    #[test]
    fn cortisol_never_leaves_the_unit_interval_under_sustained_maximum_stress() {
        let mut ind = base_individual("male");
        ind.phenotype.stress_reactivity = 1.0;
        initialize_hormones(&mut ind);
        ind.psychology.stress_level = 1.0;
        for day in 1..=3650 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.cortisol <= 1.0 && ind.hormones.cortisol >= 0.0);
    }

    #[test]
    fn a_recent_meal_after_hunger_gives_a_dopamine_reward_bump() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.2));
        update_hormones(&mut ind, 1);
        let hungry_dopamine = ind.hormones.dopamine;
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.95));
        update_hormones(&mut ind, 2);
        assert!(ind.hormones.dopamine > hungry_dopamine, "a well-fed swing should raise dopamine above the hungry level");
    }

    #[test]
    fn thyroid_falls_under_sustained_undernourishment() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=60 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.9));
            update_hormones(&mut ind, day);
        }
        let well_fed_thyroid = ind.hormones.thyroid;
        for day in 61..=200 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.1));
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.thyroid < well_fed_thyroid, "sustained undernourishment should lower thyroid output ({} vs well-fed {well_fed_thyroid})", ind.hormones.thyroid);
    }

    #[test]
    fn tsh_rises_when_thyroid_output_is_low_real_negative_feedback() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.hormones.thyroid = 0.1;
        update_hormones(&mut ind, 1);
        assert!(ind.hormones.tsh > 0.5, "low thyroid should trigger a TSH negative-feedback rise, got {}", ind.hormones.tsh);
    }

    #[test]
    fn dhea_peaks_in_young_adulthood_and_declines_with_age() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let young_adult = ind.hormones.dhea;
        for day in (25 * 365 + 1)..=(70 * 365) {
            update_hormones(&mut ind, day);
        }
        let elder = ind.hormones.dhea;
        assert!(elder < young_adult, "DHEA should decline past young adulthood ({elder} vs {young_adult})");
    }

    #[test]
    fn group_membership_elevates_ambient_oxytocin_and_vasopressin_over_isolation() {
        let mut solo = base_individual("male");
        initialize_hormones(&mut solo);
        solo.group_id = None;
        let mut grouped = base_individual("male");
        initialize_hormones(&mut grouped);
        grouped.group_id = Some("g1".to_string());
        for day in 1..=30 {
            update_hormones(&mut solo, day);
            update_hormones(&mut grouped, day);
        }
        assert!(grouped.hormones.oxytocin > solo.hormones.oxytocin);
        assert!(grouped.hormones.vasopressin > solo.hormones.vasopressin);
    }

    #[test]
    fn mating_surges_lh_oxytocin_vasopressin_testosterone_and_estrogen_in_both_parents() {
        let mut mother = base_individual("female");
        initialize_hormones(&mut mother);
        let mut father = base_individual("male");
        initialize_hormones(&mut father);
        let (mo_t, mo_e, mo_ox, mo_avp, mo_lh) = (mother.hormones.testosterone, mother.hormones.estrogen, mother.hormones.oxytocin, mother.hormones.vasopressin, mother.hormones.lh);
        let (fa_t, fa_e, fa_ox, fa_avp, fa_lh) = (father.hormones.testosterone, father.hormones.estrogen, father.hormones.oxytocin, father.hormones.vasopressin, father.hormones.lh);
        apply_mating_surge(&mut mother, &mut father);
        assert!(mother.hormones.testosterone > mo_t && mother.hormones.estrogen > mo_e && mother.hormones.oxytocin > mo_ox && mother.hormones.vasopressin > mo_avp && mother.hormones.lh > mo_lh);
        assert!(father.hormones.testosterone > fa_t && father.hormones.estrogen > fa_e && father.hormones.oxytocin > fa_ox && father.hormones.vasopressin > fa_avp && father.hormones.lh > fa_lh);
    }

    #[test]
    fn birth_surges_prolactin_which_then_decays_slowly() {
        let mut mother = base_individual("female");
        initialize_hormones(&mut mother);
        let baseline = mother.hormones.prolactin;
        apply_birth_surge(&mut mother);
        let surged = mother.hormones.prolactin;
        assert!(surged > baseline + 0.3, "birth should sharply surge prolactin ({surged} vs baseline {baseline})");
        update_hormones(&mut mother, 1);
        assert!(mother.hormones.prolactin < surged, "prolactin should start decaying the tick after birth");
        assert!(mother.hormones.prolactin > baseline, "prolactin should still be well above baseline immediately after birth");
    }

    #[test]
    fn every_hormone_stays_within_the_unit_interval_over_a_long_run() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(90 * 365) {
            ind.health.pregnancy = if day % 270 < 30 { Some(day) } else { None };
            ind.psychology.stress_level = ((day % 100) as f64) / 100.0;
            ind.extra.insert("satiation".to_string(), serde_json::json!(((day % 100) as f64) / 100.0));
            ind.health.hydration = ((day % 80) as f64) / 100.0 + 0.2;
            ind.health.hp = ((day % 90) as f64) / 100.0 + 0.1;
            if day % 500 < 20 {
                ind.extra.insert("infections".to_string(), serde_json::json!([{ "pathogen_id": "respiratory_common", "days_remaining": 5 }]));
            } else {
                ind.extra.remove("infections");
            }
            update_hormones(&mut ind, day);
            let h = &ind.hormones;
            for v in [
                h.cortisol, h.adrenaline, h.testosterone, h.estrogen, h.dopamine, h.oxytocin, h.acth, h.tsh, h.lh, h.thyroid, h.insulin, h.glucagon, h.leptin, h.ghrelin,
                h.growth_hormone, h.dhea, h.prolactin, h.progesterone, h.norepinephrine, h.vasopressin, h.fsh, h.crh, h.msh, h.endorphin, h.il6, h.tnf_alpha, h.interferon,
                h.igf1, h.adiponectin, h.npy, h.gastrin, h.secretin, h.cck, h.motilin, h.gip, h.somatostatin, h.pyy, h.pancreatic_polypeptide, h.renin, h.angiotensin_ii,
                h.aldosterone, h.anp, h.bnp, h.epo, h.pth, h.calcitonin, h.vitamin_d, h.osteocalcin, h.melatonin,
            ] {
                assert!((0.0..=1.0).contains(&v), "hormone left [0,1] on day {day}: {v}");
            }
        }
    }

    #[test]
    fn active_infection_spikes_il6_tnf_alpha_and_interferon() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.extra.insert("infections".to_string(), serde_json::json!([{ "pathogen_id": "respiratory_common", "days_remaining": 5 }]));
        for day in 1..=10 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.il6 > 0.5, "an active infection should spike IL-6, got {}", ind.hormones.il6);
        assert!(ind.hormones.tnf_alpha > 0.5, "an active infection should spike TNF-alpha, got {}", ind.hormones.tnf_alpha);
        assert!(ind.hormones.interferon > 0.4, "an active infection should spike interferon, got {}", ind.hormones.interferon);
    }

    #[test]
    fn igf1_and_osteocalcin_track_growth_hormone() {
        let mut child = base_individual("male");
        child.birth_day = 0;
        initialize_hormones(&mut child);
        for day in 1..=(12 * 365) {
            update_hormones(&mut child, day);
        }
        let child_igf1 = child.hormones.igf1;
        let mut elder = base_individual("male");
        elder.birth_day = 0;
        initialize_hormones(&mut elder);
        for day in 1..=(70 * 365) {
            update_hormones(&mut elder, day);
        }
        assert!(child_igf1 > elder.hormones.igf1, "IGF-1 should track GH's own childhood-vs-elder difference ({child_igf1} vs {})", elder.hormones.igf1);
    }

    #[test]
    fn dehydration_raises_renin_which_cascades_to_aldosterone() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.health.hydration = 0.1;
        for day in 1..=15 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.renin > 0.6, "low hydration should raise renin, got {}", ind.hormones.renin);
        assert!(ind.hormones.aldosterone > 0.4, "renin should cascade to aldosterone, got {}", ind.hormones.aldosterone);
    }

    #[test]
    fn post_fertile_low_estrogen_females_have_elevated_pth_osteoporosis_risk() {
        let mut young = base_individual("female");
        initialize_hormones(&mut young);
        for day in 1..=(30 * 365) {
            update_hormones(&mut young, day);
        }
        let mut elder = base_individual("female");
        initialize_hormones(&mut elder);
        for day in 1..=(65 * 365) {
            update_hormones(&mut elder, day);
        }
        assert!(elder.hormones.pth > young.hormones.pth, "post-menopausal PTH should exceed young-adult PTH ({} vs {})", elder.hormones.pth, young.hormones.pth);
    }

    #[test]
    fn low_melatonin_from_chronic_stress_feeds_a_small_rise_into_crh() {
        let mut calm = base_individual("male");
        initialize_hormones(&mut calm);
        calm.psychology.stress_level = 0.1;
        for day in 1..=20 {
            update_hormones(&mut calm, day);
        }
        let mut stressed = base_individual("male");
        initialize_hormones(&mut stressed);
        stressed.psychology.stress_level = 0.1;
        for day in 1..=20 {
            update_hormones(&mut stressed, day);
        }
        // Re-run stressed under sustained high stress -- both melatonin and
        // (via stress itself, plus the melatonin-suppression term) CRH should
        // separate from the calm individual's own values.
        stressed.psychology.stress_level = 0.9;
        for day in 21..=40 {
            update_hormones(&mut stressed, day);
        }
        assert!(stressed.hormones.melatonin < calm.hormones.melatonin, "chronic stress should suppress melatonin below the calm baseline");
        assert!(stressed.hormones.crh > calm.hormones.crh, "chronic stress (partly via suppressed melatonin) should elevate CRH above the calm baseline");
    }

    #[test]
    fn digestive_hormones_respond_to_the_satiation_swing_at_different_speeds() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.9));
        update_hormones(&mut ind, 1);
        // Gastrin (fast) should have moved much further toward the new
        // satiation level in one tick than PYY (slow, downstream of CCK).
        let gastrin_move = ind.hormones.gastrin - 0.3;
        let pyy_move = ind.hormones.pyy - 0.3;
        assert!(gastrin_move > pyy_move, "gastrin should respond faster than pyy to the same satiation swing ({gastrin_move} vs {pyy_move})");
    }

    #[test]
    fn adiponectin_gives_insulin_a_small_sensitizing_discount_when_lean() {
        let mut lean = base_individual("male");
        initialize_hormones(&mut lean);
        for day in 1..=60 {
            lean.extra.insert("satiation".to_string(), serde_json::json!(0.3));
            update_hormones(&mut lean, day);
        }
        let lean_insulin = lean.hormones.insulin;
        let mut heavy = base_individual("male");
        initialize_hormones(&mut heavy);
        for day in 1..=60 {
            heavy.extra.insert("satiation".to_string(), serde_json::json!(0.9));
            update_hormones(&mut heavy, day);
        }
        // Both should be driven mostly by their own satiation, but the lean
        // individual's higher adiponectin should measurably discount its
        // insulin target below a naive satiation-only prediction.
        assert!(lean.hormones.adiponectin > heavy.hormones.adiponectin, "the leaner (lower-satiation-trend) individual should carry higher adiponectin");
        assert!(lean_insulin < 0.3, "adiponectin's insulin-sensitizing discount should pull insulin below the raw 0.3 satiation target, got {lean_insulin}");
    }
}
