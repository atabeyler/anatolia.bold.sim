//! Personal-name origination for individuals born in the simulation.
//!
//! A name is not a birth gift. It is a word, exactly like any other item of
//! vocabulary, and can only exist once an individual's own lived language
//! actually supports one. `create_child` leaves `phenotype.name` at `None`;
//! this module is the only place that ever fills it in, gated by the same
//! language-stage/FOXP2/IQ thresholds `language::try_acquire_word_from_environment`
//! already uses to originate any other word, and built from the same
//! population-derived phoneme palette (see `language::derive_phoneme_palette`,
//! seeded from the two founders' own FOXP2/CNTNAP2 alleles) -- never a fixed
//! consonant/vowel list, never a fixed word-length rule, never a lookup into
//! any real-name list. A population that never develops language never
//! produces names and stays unnamed for its entire existence. That is the
//! correct, honest outcome of the experiment, not a bug to paper over.

use crate::state::Individual;
use crate::types::PhonemePalette;

/// Tries once to give `individual` a name from their own lived language.
/// No-op (returns `false`) if they already have one, if their language
/// hasn't reached deliberate vocalization (stage 2, "emotional-sounds" --
/// the same floor `try_acquire_word_from_environment` requires), or if the
/// usual FOXP2/IQ roll for originating a new word doesn't succeed this tick.
pub fn try_originate_name(individual: &mut Individual, group_id: &str, palette: &PhonemePalette) -> bool {
    if individual.phenotype.name.is_some() {
        return false;
    }
    if individual.language.stage < 2 {
        return false;
    }
    let foxp2 = individual.language.foxp2_expression;
    if foxp2 < 0.35 {
        return false;
    }
    let iq = individual.phenotype.fluid_intelligence;
    if rand::random::<f64>() > foxp2 * iq * 0.15 {
        return false;
    }
    let raw = crate::language::generate_proto_word(&individual.id, group_id, palette);
    if raw.is_empty() {
        return false;
    }
    let mut chars = raw.chars();
    let name = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => return false,
    };
    individual.phenotype.name = Some(name);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::full_palette;
    use crate::types::{Language, Phenotype};

    fn ready_ind(id: &str) -> Individual {
        Individual {
            id: id.to_string(),
            phenotype: Phenotype { fluid_intelligence: 0.8, ..Default::default() },
            language: Language { stage: 2, foxp2_expression: 0.9, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn pre_vocal_stage_never_produces_a_name() {
        let mut ind = ready_ind("a1");
        ind.language.stage = 1;
        for _ in 0..200 {
            try_originate_name(&mut ind, "g1", &full_palette());
        }
        assert!(ind.phenotype.name.is_none());
    }

    #[test]
    fn a_population_with_no_phonemes_never_names_anyone() {
        let mut ind = ready_ind("a1");
        let empty = PhonemePalette::default();
        for _ in 0..200 {
            try_originate_name(&mut ind, "g1", &empty);
        }
        assert!(ind.phenotype.name.is_none());
    }

    #[test]
    fn low_foxp2_blocks_name_origination() {
        let mut ind = ready_ind("a1");
        ind.language.foxp2_expression = 0.1;
        for _ in 0..200 {
            try_originate_name(&mut ind, "g1", &full_palette());
        }
        assert!(ind.phenotype.name.is_none());
    }

    #[test]
    fn eventually_originates_a_name_once_language_capable() {
        let mut ind = ready_ind("a1");
        let mut named = false;
        for _ in 0..2000 {
            if try_originate_name(&mut ind, "g1", &full_palette()) {
                named = true;
                break;
            }
        }
        assert!(named);
        assert!(ind.phenotype.name.as_deref().is_some_and(|n| !n.is_empty()));
    }

    #[test]
    fn already_named_individual_is_never_renamed() {
        let mut ind = ready_ind("a1");
        ind.phenotype.name = Some("Existing".to_string());
        assert!(!try_originate_name(&mut ind, "g1", &full_palette()));
        assert_eq!(ind.phenotype.name.as_deref(), Some("Existing"));
    }

    #[test]
    fn name_starts_with_an_uppercase_letter_once_originated() {
        let mut ind = ready_ind("a1");
        loop {
            if try_originate_name(&mut ind, "g1", &full_palette()) {
                break;
            }
        }
        let name = ind.phenotype.name.unwrap();
        assert!(name.chars().next().unwrap().is_uppercase());
    }

    #[test]
    fn same_id_and_group_always_originate_the_same_name() {
        let name_for = |id: &str| {
            let mut ind = ready_ind(id);
            loop {
                if try_originate_name(&mut ind, "g1", &full_palette()) {
                    return ind.phenotype.name.unwrap();
                }
            }
        };
        assert_eq!(name_for("same-id"), name_for("same-id"));
    }
}
