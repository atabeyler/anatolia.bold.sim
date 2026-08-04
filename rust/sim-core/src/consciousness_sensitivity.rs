//! Ablation/sensitivity harness for `consciousness.rs`'s growth formula.
//!
//! The formula's six terms (base_rate, lang_bonus, social_bonus, tom_bonus,
//! stress_penalty, injury_penalty) and their weights were chosen by hand,
//! not derived from data or evolved -- a fair critique of any hand-authored
//! emergent-behavior formula. This module doesn't remove that (the weights
//! are still hand-picked), but it answers a narrower, honestly answerable
//! question: *given* those weights, which terms actually matter for the
//! formula's real behavior, and by how much? A term whose ablation changes
//! nothing is decorative; a term whose ablation changes everything is
//! load-bearing. That's the evidence base a future change to make some of
//! these weights genetically evolvable (or replace a term with a
//! mechanistic sub-simulation) needs before either is worth attempting --
//! otherwise there's no way to tell whether such a change moved something
//! that mattered.
//!
//! Ablation here means literally re-running the trajectory with one term's
//! contribution forced to zero every tick (via `compute_consciousness_delta`
//! plus the same `.clamp()` `update_consciousness` itself applies) rather
//! than subtracting that term's total contribution from a baseline
//! trajectory after the fact -- the clamp makes the six-term sum non-linear
//! across many ticks (removing a positive term can change whether/when a
//! trajectory hits its ceiling and every tick's growth after that point), so
//! only a real re-run gives a correct answer.

use crate::consciousness::compute_consciousness_delta;
use crate::state::Individual;
use crate::types::{Health, Language, Mind, Phenotype, Psychology};

/// Which of the six terms to zero out for one ablation run. `None` runs the
/// unmodified formula (the baseline every ablated run is compared against).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AblatedTerm {
    BaseRate,
    LangBonus,
    SocialBonus,
    TomBonus,
    StressPenalty,
    InjuryPenalty,
}

impl AblatedTerm {
    pub const ALL: [AblatedTerm; 6] =
        [AblatedTerm::BaseRate, AblatedTerm::LangBonus, AblatedTerm::SocialBonus, AblatedTerm::TomBonus, AblatedTerm::StressPenalty, AblatedTerm::InjuryPenalty];

    pub fn label(self) -> &'static str {
        match self {
            AblatedTerm::BaseRate => "base_rate",
            AblatedTerm::LangBonus => "lang_bonus",
            AblatedTerm::SocialBonus => "social_bonus",
            AblatedTerm::TomBonus => "tom_bonus",
            AblatedTerm::StressPenalty => "stress_penalty",
            AblatedTerm::InjuryPenalty => "injury_penalty",
        }
    }
}

/// A synthetic, fixed-condition individual profile -- fixed rather than
/// itself simulated (no aging, no language-stage progression, no group
/// churn) so a trajectory run under this module isolates the consciousness
/// formula's own behavior from every other system's, matching how a
/// controlled sensitivity analysis is meant to hold everything but the
/// variable under test constant.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub name: &'static str,
    pub potential: f64,
    pub language_stage: i32,
    pub in_group: bool,
    pub theory_of_mind: i32,
    pub stress_level: f64,
    pub hp: f64,
}

/// Four profiles spanning this formula's real input range, chosen to be
/// individually recognizable against AGENTS.md's own documented ranges
/// (language stage 0-6, theory_of_mind 0-3, injury_penalty's hp<0.3 gate)
/// rather than an arbitrary grid.
pub const PROFILES: [Profile; 4] = [
    Profile { name: "newborn_isolate", potential: 0.5, language_stage: 0, in_group: false, theory_of_mind: 0, stress_level: 0.1, hp: 1.0 },
    Profile { name: "average_group_member", potential: 0.5, language_stage: 3, in_group: true, theory_of_mind: 1, stress_level: 0.2, hp: 0.9 },
    Profile { name: "high_potential_elder", potential: 0.9, language_stage: 6, in_group: true, theory_of_mind: 3, stress_level: 0.1, hp: 0.8 },
    Profile { name: "traumatized_survivor", potential: 0.6, language_stage: 2, in_group: true, theory_of_mind: 1, stress_level: 0.8, hp: 0.2 },
];

fn build_individual(profile: Profile) -> Individual {
    Individual {
        phenotype: Phenotype { consciousness_potential: profile.potential, ..Default::default() },
        mind: Mind { consciousness: 0.0, ..Default::default() },
        language: Language { stage: profile.language_stage, ..Default::default() },
        psychology: Psychology { stress_level: profile.stress_level, theory_of_mind: profile.theory_of_mind, ..Default::default() },
        health: Health { hp: profile.hp, ..Default::default() },
        group_id: if profile.in_group { Some("group_1".to_string()) } else { None },
        ..Default::default()
    }
}

/// One tick of the real formula (delta computed by `compute_consciousness_delta`,
/// same clamp `update_consciousness` applies), with `ablated` forced to zero
/// if given -- everything else identical to a real run.
fn step(consciousness: f64, ceiling: f64, ind: &Individual, ablated: Option<AblatedTerm>) -> f64 {
    let mut delta = compute_consciousness_delta(ind);
    if let Some(term) = ablated {
        match term {
            AblatedTerm::BaseRate => delta.base_rate = 0.0,
            AblatedTerm::LangBonus => delta.lang_bonus = 0.0,
            AblatedTerm::SocialBonus => delta.social_bonus = 0.0,
            AblatedTerm::TomBonus => delta.tom_bonus = 0.0,
            AblatedTerm::StressPenalty => delta.stress_penalty = 0.0,
            AblatedTerm::InjuryPenalty => delta.injury_penalty = 0.0,
        }
    }
    (consciousness + delta.sum()).clamp(0.0, ceiling)
}

/// Days for a fixed-condition profile to reach `target_fraction` of its own
/// genetic ceiling, run for at most `max_days`. Mirrors the "days to reach
/// 50% expression" style metric AGENTS.md already documents for FOXP2
/// expression, for consistency with how this codebase already reports
/// formula timescales. Returns `None` if the ceiling fraction is never
/// reached within `max_days` (a genuine, reportable outcome for a heavily
/// penalized profile/ablation combination, not an error).
pub fn days_to_reach_fraction(profile: Profile, ablated: Option<AblatedTerm>, target_fraction: f64, max_days: i32) -> Option<i32> {
    let ind = build_individual(profile);
    let ceiling = (profile.potential * 1.2).min(1.0);
    let target = ceiling * target_fraction;
    let mut consciousness = 0.0;
    for day in 1..=max_days {
        consciousness = step(consciousness, ceiling, &ind, ablated);
        if consciousness >= target {
            return Some(day);
        }
    }
    let _ = consciousness;
    None
}

/// One row of the sensitivity report: how many days a profile takes to
/// reach 50% of its ceiling, at baseline vs. with one term ablated, and the
/// resulting percentage change. A term with ~0% change for a given profile
/// is not load-bearing under that profile's conditions; a large change
/// (including a run that never reaches the target, i.e. `ablated_days: None`)
/// is.
#[derive(Debug, Clone)]
pub struct SensitivityRow {
    pub profile_name: &'static str,
    pub term: AblatedTerm,
    pub baseline_days: Option<i32>,
    pub ablated_days: Option<i32>,
    pub percent_change: Option<f64>,
}

const TARGET_FRACTION: f64 = 0.5;
const MAX_DAYS: i32 = 20_000;

/// Runs every (profile, term) ablation combination and reports the effect
/// size of each term under each profile's fixed conditions.
pub fn run_sensitivity_report() -> Vec<SensitivityRow> {
    let mut rows = Vec::new();
    for profile in PROFILES {
        let baseline_days = days_to_reach_fraction(profile, None, TARGET_FRACTION, MAX_DAYS);
        for term in AblatedTerm::ALL {
            let ablated_days = days_to_reach_fraction(profile, Some(term), TARGET_FRACTION, MAX_DAYS);
            let percent_change = match (baseline_days, ablated_days) {
                (Some(b), Some(a)) if b > 0 => Some((a - b) as f64 / b as f64 * 100.0),
                _ => None,
            };
            rows.push(SensitivityRow { profile_name: profile.name, term, baseline_days, ablated_days, percent_change });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_rate_is_load_bearing_for_every_profile() {
        // base_rate is the only term with a non-zero floor (max(.., 0.00015))
        // regardless of every other condition -- ablating it should never
        // leave a profile's time-to-50% unchanged, since every other term
        // can independently be zero (isolated newborn: no group, no ToM, no
        // stress, no injury).
        for profile in PROFILES {
            let baseline = days_to_reach_fraction(profile, None, TARGET_FRACTION, MAX_DAYS);
            let ablated = days_to_reach_fraction(profile, Some(AblatedTerm::BaseRate), TARGET_FRACTION, MAX_DAYS);
            assert_ne!(baseline, ablated, "expected base_rate ablation to change the trajectory for profile {}", profile.name);
        }
    }

    #[test]
    fn social_bonus_has_no_effect_on_a_profile_that_is_never_in_a_group() {
        let isolate = PROFILES[0];
        assert!(!isolate.in_group, "test fixture assumption: newborn_isolate must not be in a group");
        let baseline = days_to_reach_fraction(isolate, None, TARGET_FRACTION, MAX_DAYS);
        let ablated = days_to_reach_fraction(isolate, Some(AblatedTerm::SocialBonus), TARGET_FRACTION, MAX_DAYS);
        assert_eq!(baseline, ablated, "social_bonus is already zero for a non-group individual, so ablating it must be a no-op");
    }

    #[test]
    fn injury_penalty_only_matters_below_the_hp_gate() {
        // injury_penalty is gated (`if hp < 0.3`), so a healthy profile
        // (hp=0.9, average_group_member) must be unaffected by its ablation,
        // while the traumatized_survivor profile (hp=0.2, below the gate)
        // must be measurably affected.
        let healthy = PROFILES[1];
        assert!(healthy.hp >= 0.3, "test fixture assumption: average_group_member must be above the injury_penalty gate");
        let baseline = days_to_reach_fraction(healthy, None, TARGET_FRACTION, MAX_DAYS);
        let ablated = days_to_reach_fraction(healthy, Some(AblatedTerm::InjuryPenalty), TARGET_FRACTION, MAX_DAYS);
        assert_eq!(baseline, ablated, "injury_penalty is already zero above the hp<0.3 gate, so ablating it must be a no-op");

        let injured = PROFILES[3];
        assert!(injured.hp < 0.3, "test fixture assumption: traumatized_survivor must be below the injury_penalty gate");
        let baseline = days_to_reach_fraction(injured, None, TARGET_FRACTION, MAX_DAYS);
        let ablated = days_to_reach_fraction(injured, Some(AblatedTerm::InjuryPenalty), TARGET_FRACTION, MAX_DAYS);
        assert_ne!(baseline, ablated, "expected injury_penalty ablation to speed up a profile below the hp<0.3 gate");
    }

    #[test]
    fn removing_a_penalty_never_slows_a_trajectory_down() {
        // stress_penalty and injury_penalty are subtracted; ablating either
        // (forcing it to zero) can only leave consciousness growth the same
        // or faster each tick, never slower -- so days-to-target can only
        // stay the same or shrink, never grow, for every profile.
        for profile in PROFILES {
            let baseline = days_to_reach_fraction(profile, None, TARGET_FRACTION, MAX_DAYS);
            for term in [AblatedTerm::StressPenalty, AblatedTerm::InjuryPenalty] {
                let ablated = days_to_reach_fraction(profile, Some(term), TARGET_FRACTION, MAX_DAYS);
                match (baseline, ablated) {
                    (Some(b), Some(a)) => assert!(a <= b, "{} ablation made {} slower ({a} > {b} days)", term.label(), profile.name),
                    (None, Some(_)) => {} // baseline never reached target, ablation did -- consistent (faster)
                    (Some(_), None) => panic!("{} ablation made {} never reach the target when baseline did", term.label(), profile.name),
                    (None, None) => {}
                }
            }
        }
    }

    #[test]
    fn removing_a_bonus_never_speeds_a_trajectory_up() {
        // Mirror of the penalty test above: base_rate, lang_bonus,
        // social_bonus, tom_bonus are added; ablating one can only leave a
        // trajectory the same or slower, never faster.
        for profile in PROFILES {
            let baseline = days_to_reach_fraction(profile, None, TARGET_FRACTION, MAX_DAYS);
            for term in [AblatedTerm::BaseRate, AblatedTerm::LangBonus, AblatedTerm::SocialBonus, AblatedTerm::TomBonus] {
                let ablated = days_to_reach_fraction(profile, Some(term), TARGET_FRACTION, MAX_DAYS);
                match (baseline, ablated) {
                    (Some(b), Some(a)) => assert!(a >= b, "{} ablation made {} faster ({a} < {b} days)", term.label(), profile.name),
                    (Some(_), None) => {} // baseline reached target, ablation didn't -- consistent (slower)
                    (None, Some(_)) => panic!("{} ablation made {} reach the target when baseline never did", term.label(), profile.name),
                    (None, None) => {}
                }
            }
        }
    }

    #[test]
    fn run_sensitivity_report_covers_every_profile_and_term() {
        let rows = run_sensitivity_report();
        assert_eq!(rows.len(), PROFILES.len() * AblatedTerm::ALL.len());
    }
}
