use sim_core::{combine_gametes, create_gamete, create_genome, create_founder, Individual};

/// The DB payload optimization (state.rs's serialize_slim_genome /
/// deserialize_hydrated_genome) must be lossless: an individual's genome
/// after a JSON round-trip has to be indistinguishable from before, for every
/// field the rest of the engine actually reads (chromosome/expression_type/
/// trait_name/locus_id), not just the alleles that are genuinely
/// per-individual and still transmitted directly.
fn roundtrip(individual: &Individual) -> Individual {
    let json = serde_json::to_string(individual).expect("individual should serialize");
    serde_json::from_str(&json).expect("individual should deserialize")
}

/// serde_json (without the `float_roundtrip` feature, which this workspace
/// doesn't enable) can shift an f64 by a single ULP on the way through its
/// default shortest-round-trip formatter -- true of any f64 field in this
/// codebase, not something the genome-slimming change introduces. Compare
/// with a tight epsilon instead of exact equality so the test verifies the
/// thing it's actually meant to (values pass through, not bit-identically).
fn approx_eq(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => (a - b).abs() < 1e-9,
        (None, None) => true,
        _ => false,
    }
}

#[test]
fn a_founders_genome_survives_a_json_roundtrip_unchanged() {
    let founder = create_founder(&serde_json::json!({ "sex": "female", "ageYears": 22, "x": 1.0, "y": 2.0, "name": "Ada" }));
    let restored = roundtrip(&founder);

    assert_eq!(restored.genome.len(), founder.genome.len());
    for (locus_id, locus) in &founder.genome {
        let restored_locus = restored.genome.get(locus_id).expect("locus present after roundtrip");
        assert_eq!(restored_locus.locus_id, locus.locus_id, "{locus_id} locus_id mismatch");
        assert_eq!(restored_locus.chromosome, locus.chromosome, "{locus_id} chromosome mismatch");
        assert_eq!(restored_locus.trait_name, locus.trait_name, "{locus_id} trait_name mismatch");
        assert_eq!(restored_locus.expression_type, locus.expression_type, "{locus_id} expression_type mismatch");
        assert!(approx_eq(restored_locus.allele1.value, locus.allele1.value), "{locus_id} allele1 mismatch");
        assert!(approx_eq(restored_locus.allele2.value, locus.allele2.value), "{locus_id} allele2 mismatch");
        assert_eq!(restored_locus.allele1.origin, locus.allele1.origin, "{locus_id} allele1 origin mismatch");
        assert_eq!(restored_locus.allele2.origin, locus.allele2.origin, "{locus_id} allele2 origin mismatch");
    }
}

/// MAOA_01 is the one locus whose `expression_type` is NOT a pure function of
/// locus_id -- it's "x_linked" in the static table but becomes "hemizygous"
/// specifically for a male child's single inherited allele. This is the case
/// serialize_slim_genome/hydrate_genome_metadata has to get right without
/// storing expression_type on the wire at all.
#[test]
fn a_sons_hemizygous_x_linked_locus_survives_a_json_roundtrip() {
    let g1 = create_gamete(&create_genome(None), 0.0);
    let g2 = create_gamete(&create_genome(None), 0.0);
    let son_genome = combine_gametes(&g1, &g2, "male");
    assert_eq!(son_genome["MAOA_01"].expression_type, "hemizygous");
    assert!(son_genome["MAOA_01"].allele2.value.is_none());

    let mut son = Individual { sex: "male".to_string(), ..Default::default() };
    son.genome = son_genome;
    let restored = roundtrip(&son);

    assert_eq!(restored.genome["MAOA_01"].expression_type, "hemizygous");
    assert!(restored.genome["MAOA_01"].allele2.value.is_none());
    assert_eq!(restored.genome["MAOA_01"].chromosome.as_deref(), Some("X"));
    assert!(approx_eq(restored.genome["MAOA_01"].allele1.value, son.genome["MAOA_01"].allele1.value));
}

/// A female's MAOA_01 stays plain "x_linked" (both alleles present) --
/// confirms the hemizygous branch in hydrate_genome_metadata doesn't
/// misfire for the non-hemizygous case.
#[test]
fn a_daughters_x_linked_locus_stays_x_linked_not_hemizygous_after_roundtrip() {
    let g1 = create_gamete(&create_genome(None), 0.0);
    let g2 = create_gamete(&create_genome(None), 0.0);
    let daughter_genome = combine_gametes(&g1, &g2, "female");
    assert_eq!(daughter_genome["MAOA_01"].expression_type, "x_linked");
    assert!(daughter_genome["MAOA_01"].allele2.value.is_some());

    let mut daughter = Individual { sex: "female".to_string(), ..Default::default() };
    daughter.genome = daughter_genome;
    let restored = roundtrip(&daughter);

    assert_eq!(restored.genome["MAOA_01"].expression_type, "x_linked");
    assert!(restored.genome["MAOA_01"].allele2.value.is_some());
}

/// Regression test for the founder-genetics bug: create_founder used to
/// build every male founder's genome as diploid at every locus, X-linked
/// MAOA_01 included -- unlike a non-founder son (built via
/// combine_gametes, already covered above), whose hemizygous X was
/// always modeled correctly. A male founder's own single X must now
/// match that same representation instead of being averaged as if he
/// had two independent copies.
#[test]
fn a_male_founders_x_linked_locus_is_hemizygous_not_diploid() {
    let founder = create_founder(&serde_json::json!({ "sex": "male", "ageYears": 22, "x": 1.0, "y": 2.0, "name": "Adam" }));
    assert_eq!(founder.genome["MAOA_01"].expression_type, "hemizygous");
    assert!(founder.genome["MAOA_01"].allele2.value.is_none());
    assert_eq!(founder.genome["MAOA_01"].chromosome.as_deref(), Some("X"));
    // Every autosomal locus stays diploid -- only X-linked ones collapse.
    assert_eq!(founder.genome["BDNF_01"].expression_type, "codominant");
    assert!(founder.genome["BDNF_01"].allele2.value.is_some());

    let restored = roundtrip(&founder);
    assert_eq!(restored.genome["MAOA_01"].expression_type, "hemizygous");
    assert!(restored.genome["MAOA_01"].allele2.value.is_none());
}

/// A female founder's own two X chromosomes are both real -- confirms the
/// fix doesn't overreach and start collapsing MAOA_01 for founders who
/// actually are diploid at that locus.
#[test]
fn a_female_founders_x_linked_locus_stays_diploid() {
    let founder = create_founder(&serde_json::json!({ "sex": "female", "ageYears": 22, "x": 1.0, "y": 2.0, "name": "Ada" }));
    assert_eq!(founder.genome["MAOA_01"].expression_type, "x_linked");
    assert!(founder.genome["MAOA_01"].allele2.value.is_some());
}

/// The whole point of slimming the wire format -- confirms the optimization
/// is actually doing something, not just a no-op refactor. Compares the
/// `genome` key's serialized size inside a real `Individual` (which goes
/// through `serialize_slim_genome`) against the same locus set serialized
/// with every field present (the pre-optimization shape).
#[test]
fn the_slim_wire_format_is_meaningfully_smaller_than_a_naive_full_dump() {
    let founder = create_founder(&serde_json::json!({ "sex": "male", "ageYears": 22, "x": 1.0, "y": 2.0, "name": "Adam" }));

    let individual_json = serde_json::to_value(&founder).expect("individual should serialize");
    let slim_genome_len = serde_json::to_vec(&individual_json["genome"]).unwrap().len();

    let full_len: usize = founder
        .genome
        .values()
        .map(|locus| {
            serde_json::to_string(&serde_json::json!({
                "locusId": locus.locus_id,
                "chromosome": locus.chromosome,
                "allele1": locus.allele1,
                "allele2": locus.allele2,
                "expressionType": locus.expression_type,
                "trait": locus.trait_name,
            }))
            .unwrap()
            .len()
        })
        .sum();

    assert!(
        slim_genome_len < full_len * 7 / 10,
        "expected slim genome ({slim_genome_len} bytes) to be at least 30% smaller than the naive full dump ({full_len} bytes)"
    );
}
